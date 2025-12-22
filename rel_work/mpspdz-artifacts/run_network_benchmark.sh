#!/bin/bash

# Script to run MP-SPDZ Vickrey auction with netem RTT emulation
# Usage: sudo ./run_network_benchmark.sh <num_bidders> <rtt_ms> <num_runs>

set -e

NUM_BIDDERS=${1:-100}
RTT_MS=${2:-20}
NUM_RUNS=${3:-3}

# Calculate one-way delay (RTT/2)
DELAY_MS=$((RTT_MS / 2))

echo "=========================================="
echo "MP-SPDZ Network Benchmark"
echo "=========================================="
echo "Number of bidders: $NUM_BIDDERS"
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

# Change to MP-SPDZ directory
cd "$(dirname "$0")"

# Check if compiled binary exists
if [ ! -f "mascot-party.x" ]; then
    echo "Error: mascot-party.x not found. Please compile first:"
    echo "  ./compile.py vickrey"
    exit 1
fi

# Check if program is compiled
if ! ls Programs/Bytecode/vickrey-*.bc >/dev/null 2>&1; then
    echo "Error: Vickrey program not compiled. Please compile first:"
    echo "  ./compile.py vickrey"
    exit 1
fi

# Results directory
RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"
RESULTS_FILE="$RESULTS_DIR/mpspdz_network_${NUM_BIDDERS}_${RTT_MS}ms.csv"

# Write CSV header
echo "run,online_time_s,comm_bytes_mb,comm_bytes_kb" > "$RESULTS_FILE"

echo ""
echo "Starting benchmark runs..."
echo ""

for run in $(seq 1 $NUM_RUNS); do
    echo "=== Run $run/$NUM_RUNS ==="
    
    # Clear persistence files between runs
    ./clear_persist.sh 2>/dev/null || true
    
    # Run MP-SPDZ with MASCOT protocol
    # Output goes to logs/vickrey-mascot-party.x-N2-0 and logs/vickrey-mascot-party.x-N2-1
    BENCH=1 Scripts/mascot.sh -v vickrey > /tmp/mpspdz_run${run}.log 2>&1
    
    if [ $? -ne 0 ]; then
        echo "Error: Run $run failed"
        echo "Check logs: /tmp/mpspdz_run${run}.log"
        continue
    fi
    
    # Parse results from logs
    RESULT_LINE=$(python3 << PYTHON_SCRIPT
import re
import sys
import glob

RUN_NUM = $run

def parse_logs():
    online_time = None
    comm_bytes = None
    
    # Look for log files
    log_files = glob.glob(f'logs/vickrey-mascot-party.x-N2-*')
    if not log_files:
        log_files = glob.glob(f'logs/vickrey-*')
    
    # Try to find timing and communication info
    for log_file in sorted(log_files):
        try:
            with open(log_file, 'r') as f:
                content = f.read()
            
            # Look for timing information
            # MP-SPDZ outputs: "Time = X seconds"
            time_match = re.search(r'Time\s*=\s*([\d.]+)\s*seconds?', content, re.IGNORECASE)
            if time_match:
                online_time = float(time_match.group(1))
            
            # Look for communication
            # MP-SPDZ outputs: "Global data sent = X MB (all parties)"
            # We want the global (total) communication, not just party 0
            comm_match = re.search(r'Global data sent\s*=\s*([\d.]+)\s*MB', content, re.IGNORECASE)
            if comm_match:
                comm_mb = float(comm_match.group(1))
                comm_bytes = int(comm_mb * 1024 * 1024)
            
            # If we found both, we're done
            if online_time is not None and comm_bytes is not None:
                break
                
        except Exception as e:
            continue
    
    return online_time, comm_bytes

online_time, comm_bytes = parse_logs()

if online_time is not None and comm_bytes is not None:
    comm_mb = comm_bytes / (1024.0 * 1024.0)
    comm_kb = comm_bytes / 1024.0
    print(f"{RUN_NUM},{online_time:.6f},{comm_mb:.2f},{comm_kb:.2f}")
else:
    # If parsing failed, try to extract from combined log
    try:
        with open(f'/tmp/mpspdz_run{RUN_NUM}.log', 'r') as f:
            content = f.read()
        
        # Try same patterns on combined log
        time_match = re.search(r'Time\s*=\s*([\d.]+)\s*seconds?', content, re.IGNORECASE)
        comm_match = re.search(r'Global data sent\s*=\s*([\d.]+)\s*MB', content, re.IGNORECASE)
        
        if time_match:
            online_time = float(time_match.group(1))
        if comm_match:
            comm_mb = float(comm_match.group(1))
            comm_bytes = int(comm_mb * 1024 * 1024)
        
        if online_time is not None and comm_bytes is not None:
            comm_mb = comm_bytes / (1024.0 * 1024.0)
            comm_kb = comm_bytes / 1024.0
            print(f"{RUN_NUM},{online_time:.6f},{comm_mb:.2f},{comm_kb:.2f}")
        else:
            print(f"{RUN_NUM},ERROR,ERROR,ERROR", file=sys.stderr)
    except:
        print(f"{RUN_NUM},ERROR,ERROR,ERROR", file=sys.stderr)
PYTHON_SCRIPT
    )
    
    if [ -n "$RESULT_LINE" ] && [[ ! "$RESULT_LINE" =~ ERROR ]]; then
        echo "$RESULT_LINE" >> "$RESULTS_FILE"
        echo "  Result: $RESULT_LINE"
    else
        echo "  Warning: Failed to parse results for run $run"
        echo "  Check logs in logs/ directory"
    fi
    
    # Small delay between runs
    sleep 1
done

echo ""
echo "Results saved to: $RESULTS_FILE"
echo "Done!"

