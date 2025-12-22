#!/bin/bash

# Script to run Obsidian network benchmarks with netem RTT emulation
# Usage: ./run_network_benchmark.sh <num_clients> <domain_size> <rtt_ms> <num_runs>

set -e

NUM_CLIENTS=${1:-100}
DOMAIN_SIZE=${2:-1024}
RTT_MS=${3:-20}
NUM_RUNS=${4:-5}

# Calculate one-way delay (RTT/2)
DELAY_MS=$((RTT_MS / 2))

echo "=========================================="
echo "Obsidian Network Benchmark"
echo "=========================================="
echo "Number of bidders: $NUM_CLIENTS"
echo "Domain size: $DOMAIN_SIZE"
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

# Build the binaries if needed
cd "$(dirname "$0")"
if [ ! -f "target/release/party0" ] || [ ! -f "target/release/party1" ]; then
    echo "Building party0 and party1 (socket-based network implementation)..."
    export RUSTFLAGS="-C target-cpu=native"
    cargo build --release --bin party0 --bin party1
fi

# Note: We use party0/party1 (socket-based) instead of dpf_benchmark_network 
# (which uses sleep() simulation) because we need actual network RTT via netem

# Results file
RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"
RESULTS_FILE="$RESULTS_DIR/network_benchmark_${NUM_CLIENTS}_${DOMAIN_SIZE}_${RTT_MS}ms.csv"

# Write CSV header
echo "run,preprocess_time_ms,online_time_ms,preprocess_comm_bytes,online_comm_bytes" > "$RESULTS_FILE"

echo ""
echo "Starting benchmark runs..."
echo ""

for run in $(seq 1 $NUM_RUNS); do
    echo "=== Run $run/$NUM_RUNS ==="
    
    # Start party0 in background
    timeout 300 ./target/release/party0 $NUM_CLIENTS $DOMAIN_SIZE > /tmp/party0_run${run}.log 2>&1 &
    PARTY0_PID=$!
    
    # Small delay to let party0 start listening
    sleep 0.1
    
    # Start party1 (this will connect to party0)
    timeout 300 ./target/release/party1 $NUM_CLIENTS $DOMAIN_SIZE > /tmp/party1_run${run}.log 2>&1 &
    PARTY1_PID=$!
    
    # Wait for both to complete
    wait $PARTY0_PID
    PARTY0_EXIT=$?
    wait $PARTY1_PID
    PARTY1_EXIT=$?
    
    if [ $PARTY0_EXIT -ne 0 ] || [ $PARTY1_EXIT -ne 0 ]; then
        echo "Error: Run $run failed (party0: $PARTY0_EXIT, party1: $PARTY1_EXIT)"
        echo "Check logs: /tmp/party0_run${run}.log and /tmp/party1_run${run}.log"
        continue
    fi
    
    # Parse results from logs and append to CSV
    RESULT_LINE=$(python3 << PYTHON_SCRIPT
import re
import sys

RUN_NUM = $run

def parse_log(log_file):
    preprocess_time = None
    online_time = None
    preprocess_comm = None
    online_comm = None
    
    try:
        with open(log_file, 'r') as f:
            content = f.read()
            
        # Extract from BENCHMARK SUMMARY section
        preprocess_match = re.search(r'PREPROCESS_TIME_MS: ([\d.]+)', content)
        if preprocess_match:
            preprocess_time = float(preprocess_match.group(1))
        
        online_match = re.search(r'ONLINE_TIME_MS: ([\d.]+)', content)
        if online_match:
            online_time = float(online_match.group(1))
        
        preprocess_comm_match = re.search(r'PREPROCESS_COMM_BYTES: (\d+)', content)
        if preprocess_comm_match:
            preprocess_comm = int(preprocess_comm_match.group(1))
        
        online_comm_match = re.search(r'ONLINE_COMM_BYTES: (\d+)', content)
        if online_comm_match:
            online_comm = int(online_comm_match.group(1))
            
    except Exception as e:
        print(f"Error parsing {log_file}: {e}", file=sys.stderr)
    
    return preprocess_time, online_time, preprocess_comm, online_comm

# Try party0 log first (has communication stats)
preprocess_time, online_time, preprocess_comm, online_comm = parse_log(f'/tmp/party0_run{RUN_NUM}.log')

# If not found, try party1
if preprocess_time is None:
    preprocess_time, online_time, preprocess_comm, online_comm = parse_log(f'/tmp/party1_run{RUN_NUM}.log')

if preprocess_time is not None and online_time is not None:
    print(f"{RUN_NUM},{preprocess_time:.3f},{online_time:.3f},{preprocess_comm or 0},{online_comm or 0}")
else:
    print(f"{RUN_NUM},ERROR,ERROR,ERROR,ERROR", file=sys.stderr)
PYTHON_SCRIPT
    )
    
    if [ -n "$RESULT_LINE" ] && [[ ! "$RESULT_LINE" =~ ERROR ]]; then
        echo "$RESULT_LINE" >> "$RESULTS_FILE"
        echo "  Result: $RESULT_LINE"
    else
        echo "  Warning: Failed to parse results for run $run"
    fi
    
    # Small delay between runs
    sleep 0.5
done

echo ""
echo "Results saved to: $RESULTS_FILE"
echo "Done!"

