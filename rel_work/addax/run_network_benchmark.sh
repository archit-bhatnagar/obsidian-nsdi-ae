#!/bin/bash

# Script to run Addax non-interactive auction with netem RTT emulation
# Usage: sudo ./run_network_benchmark.sh <num_bidders> <bucket_num> <rtt_ms> <num_runs>

set -e

NUM_BIDDERS=${1:-100}
BUCKET_NUM=${2:-10000}
RTT_MS=${3:-20}
NUM_RUNS=${4:-3}

# Calculate one-way delay (RTT/2)
DELAY_MS=$((RTT_MS / 2))

echo "=========================================="
echo "Addax Non-Interactive Network Benchmark"
echo "=========================================="
echo "Number of bidders: $NUM_BIDDERS"
echo "Bucket number: $BUCKET_NUM"
echo "RTT: ${RTT_MS}ms (delay: ${DELAY_MS}ms)"
echo "Number of runs: $NUM_RUNS"
echo "=========================================="

# Check if running as root (needed for netem)
if [ "$EUID" -ne 0 ]; then 
    echo "Error: This script must be run as root to configure netem"
    exit 1
fi

# Interface to use (loopback for local testing)
INTERFACE="lo"

# Clean up any existing netem rules
echo "Cleaning up existing netem rules..."
tc qdisc del dev $INTERFACE root 2>/dev/null || true

# Set up netem with delay
echo "Setting up netem: ${DELAY_MS}ms delay on $INTERFACE..."
tc qdisc add dev $INTERFACE root netem delay ${DELAY_MS}ms

# Function to cleanup netem on exit
cleanup() {
    echo ""
    echo "Cleaning up netem..."
    tc qdisc del dev $INTERFACE root 2>/dev/null || true
    echo "Cleanup complete"
}
trap cleanup EXIT

# Change to Addax directory
cd "$(dirname "$0")/auction/auction-local-computation"

# Check if binary exists
if [ ! -f "build/auction-non-interactive" ]; then
    echo "Error: auction-non-interactive binary not found. Please build it first."
    exit 1
fi

# Check if share files exist
SHARE_DIR="../tools/build"
S1_FILE="$SHARE_DIR/${NUM_BIDDERS}-${BUCKET_NUM}-s1-idx"
S2_FILE="$SHARE_DIR/${NUM_BIDDERS}-${BUCKET_NUM}-s2-idx"

# Try different share directory naming conventions
# First try: shares_BUCKET_BIDDERS (e.g., shares_1000_25)
# Then try: shares_BUCKET (e.g., shares_10000) for existing 100-bidder configs
# Finally try: shares/ (for 100-bidder configs with 1000 buckets)
if [ -d "$SHARE_DIR/shares_${BUCKET_NUM}_${NUM_BIDDERS}" ]; then
    SHARES_DIR="$SHARE_DIR/shares_${BUCKET_NUM}_${NUM_BIDDERS}"
elif [ -d "$SHARE_DIR/shares_${BUCKET_NUM}" ]; then
    SHARES_DIR="$SHARE_DIR/shares_${BUCKET_NUM}"
elif [ -d "$SHARE_DIR/shares" ] && [ $NUM_BIDDERS -eq 100 ]; then
    # For 100 bidders, shares might be in the generic "shares" directory
    SHARES_DIR="$SHARE_DIR/shares"
else
    echo "Warning: Share directory not found, trying shares_${BUCKET_NUM}_${NUM_BIDDERS}"
    SHARES_DIR="$SHARE_DIR/shares_${BUCKET_NUM}_${NUM_BIDDERS}"
fi

if [ ! -f "$S1_FILE" ] || [ ! -f "$S2_FILE" ] || [ ! -d "$SHARES_DIR" ]; then
    echo "Error: Share files not found for ${NUM_BIDDERS} bidders, ${BUCKET_NUM} buckets"
    echo "S1: $S1_FILE"
    echo "S2: $S2_FILE"
    echo "Shares dir: $SHARES_DIR"
    echo "Please generate shares first using tools/adv-gen-sc.sh"
    exit 1
fi

# Results directory
RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"
RESULTS_FILE="$RESULTS_DIR/addax_network_${NUM_BIDDERS}_${BUCKET_NUM}_${RTT_MS}ms.csv"

# Write CSV header
echo "run,online_time_s,comm_bytes_mb,comm_bytes_kb" > "$RESULTS_FILE"

echo ""
echo "Starting benchmark runs..."
echo ""

# Ports for communication
PUB_PORT=6666

for run in $(seq 1 $NUM_RUNS); do
    echo "=== Run $run/$NUM_RUNS ==="
    
    # Start publisher first (it listens on ports)
    timeout 300 ./build/auction-non-interactive -a $NUM_BIDDERS -b $BUCKET_NUM \
        -s "$S1_FILE" -S "$S2_FILE" -d "$SHARES_DIR" \
        -i 127.0.0.1 -p $PUB_PORT > /tmp/addax_publisher_run${run}.log 2>&1 &
    PUBLISHER_PID=$!
    
    # Small delay to let publisher start listening
    sleep 0.5
    
    # Start server (it connects to publisher)
    timeout 300 ./build/auction-non-interactive -k -a $NUM_BIDDERS -b $BUCKET_NUM \
        -s "$S1_FILE" -S "$S2_FILE" -d "$SHARES_DIR" \
        -i 127.0.0.1 -p $PUB_PORT > /tmp/addax_server_run${run}.log 2>&1 &
    SERVER_PID=$!
    
    # Wait for both to complete
    wait $PUBLISHER_PID
    PUBLISHER_EXIT=$?
    wait $SERVER_PID
    SERVER_EXIT=$?
    
    if [ $SERVER_EXIT -ne 0 ] || [ $PUBLISHER_EXIT -ne 0 ]; then
        echo "Error: Run $run failed (server: $SERVER_EXIT, publisher: $PUBLISHER_EXIT)"
        echo "Check logs: /tmp/addax_server_run${run}.log and /tmp/addax_publisher_run${run}.log"
        continue
    fi
    
    # Parse results from logs (publisher has the final summary)
    RESULT_LINE=$(python3 << PYTHON_SCRIPT
import re
import sys

RUN_NUM = $run

def parse_log(log_file):
    online_time = None
    comm_bytes = None
    
    try:
        with open(log_file, 'r') as f:
            content = f.read()
            
        # Extract total time (this is the online time)
        time_match = re.search(r'TIME: total:\s+([\d.]+)', content)
        if time_match:
            online_time = float(time_match.group(1))
        
        # Extract grand total communication
        comm_match = re.search(r'GRAND TOTAL:\s+(\d+)\s+bytes', content)
        if comm_match:
            comm_bytes = int(comm_match.group(1))
            
    except Exception as e:
        print(f"Error parsing {log_file}: {e}", file=sys.stderr)
    
    return online_time, comm_bytes

# Try publisher log first (has final summary)
online_time, comm_bytes = parse_log(f'/tmp/addax_publisher_run{RUN_NUM}.log')

# If not found, try server log
if online_time is None:
    online_time, comm_bytes = parse_log(f'/tmp/addax_server_run{RUN_NUM}.log')

if online_time is not None and comm_bytes is not None:
    comm_mb = comm_bytes / (1024.0 * 1024.0)
    comm_kb = comm_bytes / 1024.0
    print(f"{RUN_NUM},{online_time:.6f},{comm_mb:.2f},{comm_kb:.2f}")
else:
    print(f"{RUN_NUM},ERROR,ERROR,ERROR", file=sys.stderr)
PYTHON_SCRIPT
    )
    
    if [ -n "$RESULT_LINE" ] && [[ ! "$RESULT_LINE" =~ ERROR ]]; then
        echo "$RESULT_LINE" >> "$RESULTS_FILE"
        echo "  Result: $RESULT_LINE"
    else
        echo "  Warning: Failed to parse results for run $run"
    fi
    
    # Small delay between runs
    sleep 1
done

echo ""
echo "Results saved to: $RESULTS_FILE"
echo "Done!"

