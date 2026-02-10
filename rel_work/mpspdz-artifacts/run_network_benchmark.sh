#!/bin/bash

# Script to run MP-SPDZ Vickrey auction with netem RTT emulation
# Usage: sudo ./run_network_benchmark.sh <num_bidders> <domain_size> <rtt_ms> <num_runs>
# Note: domain_size determines the max bid value

set -e

NUM_BIDDERS=${1:-100}
DOMAIN_SIZE=${2:-1000}
RTT_MS=${3:-20}
NUM_RUNS=${4:-3}

# Calculate one-way delay (RTT/2)
DELAY_MS=$((RTT_MS / 2))

echo "=========================================="
echo "MP-SPDZ Network Benchmark"
echo "=========================================="
echo "Number of bidders: $NUM_BIDDERS"
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

# Change to MP-SPDZ directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Adjust NUM_BIDDERS (not domain) to ensure n_per_thread is even
# The vickrey program requires n_inputs // n_threads to be divisible by 2
ADJUSTED_BIDDERS=$(python3 << PYTHON_ADJUST
import math
import sys

n_inputs = $NUM_BIDDERS

# Calculate n_threads using the logic: 2^(log2(n_inputs) - 4)
# Note: n_inputs here is bidders, NOT domain size
try:
    log_val = math.log(n_inputs, 2)
    # Using offset -4 as requested
    power_val = int(log_val - 4)
    # Ensure power_val is at least 0 (1 thread minimum)
    if power_val < 0:
        power_val = 0
    n_threads = int(math.ceil(2 ** power_val))
except:
    n_threads = 1

if n_threads < 1:
    n_threads = 1

n_per_thread = n_inputs // n_threads

# If n_per_thread is odd, adjust n_inputs to make it even
if n_per_thread % 2 != 0:
    # Round up to next even multiple of n_threads
    adjusted = ((n_inputs // n_threads) + 1) * n_threads
    # If that made it odd (e.g. if n_threads is odd?), ensure multiple of 2*n_threads
    if (adjusted // n_threads) % 2 != 0:
         adjusted += n_threads
    print(adjusted)
else:
    print(n_inputs)
PYTHON_ADJUST
)

if [ "$ADJUSTED_BIDDERS" != "$NUM_BIDDERS" ]; then
    echo "Note: Adjusted number of bidders from $NUM_BIDDERS to $ADJUSTED_BIDDERS (required for threads)"
    ACTUAL_BIDDERS=$ADJUSTED_BIDDERS
else
    ACTUAL_BIDDERS=$NUM_BIDDERS
fi

# Generate input files: Count=ACTUAL_BIDDERS, Max=DOMAIN_SIZE
echo ""
echo "Generating input files for $ACTUAL_BIDDERS bidders (max bid $DOMAIN_SIZE)..."
python3 generate_inputs.py $ACTUAL_BIDDERS $DOMAIN_SIZE Player-Data

# Program name based on ACTUAL_BIDDERS
PROG_NAME="vickrey${ACTUAL_BIDDERS}"
VICKREY_MPC="Programs/Source/vickrey.mpc"

# Check if source file exists
if [ ! -f "$VICKREY_MPC" ]; then
    echo "Error: $VICKREY_MPC not found"
    exit 1
fi

# Function to toggle write/read mode in vickrey.mpc
toggle_mode() {
    local mpc_file=$1
    local mode=$2  # "write" or "read"
    
    if [ "$mode" == "write" ]; then
        # Uncomment write, comment read
        sed -i 's/^# input.write_to_file()/input.write_to_file()/' "$mpc_file"
        sed -i 's/^ret = bids_from_file.read_from_file(start= 0)/# ret = bids_from_file.read_from_file(start= 0)/' "$mpc_file"
    else
        # Comment write, uncomment read
        sed -i 's/^input.write_to_file()/# input.write_to_file()/' "$mpc_file"
        sed -i 's/^# ret = bids_from_file.read_from_file(start= 0)/ret = bids_from_file.read_from_file(start= 0)/' "$mpc_file"
    fi
}

# Copy source file and create bidder-specific version
echo ""
echo "Creating program $PROG_NAME..."
PROG_MPC="Programs/Source/${PROG_NAME}.mpc"
cp "$VICKREY_MPC" "$PROG_MPC"

# Compile program
echo "Compiling program as $PROG_NAME..."
./compile.py "$PROG_NAME" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo "Error: Failed to compile $PROG_NAME"
    exit 1
fi

# Check if compiled binary exists
if [ ! -f "mascot-party.x" ]; then
    echo "Error: mascot-party.x not found. Please build MP-SPDZ first"
    exit 1
fi

# Results directory
RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"
# Use the ORIGINAL requested number (NUM_BIDDERS) for the filename, not the adjusted one
RESULTS_FILE="$RESULTS_DIR/mpspdz_network_${NUM_BIDDERS}_${DOMAIN_SIZE}_${RTT_MS}ms.csv"

# Write CSV header
echo "run,preprocessing_time_s,online_time_s,online_comm_mb" > "$RESULTS_FILE"

echo ""
echo "Generating persistence shares (preprocessing phase)..."
echo ""

# Step 1: Generate persistence shares (write mode)
echo "Switching to write mode for persistence generation..."
toggle_mode "$PROG_MPC" "write"
./compile.py "$PROG_NAME" > /dev/null 2>&1

# Clear persistence files first
./clear_persist.sh 2>/dev/null || true

# Run preprocessing phase (generates persistence)
echo "Running preprocessing to generate persistence shares..."
BENCH=1 Scripts/mascot.sh -v "$PROG_NAME" > /tmp/mpspdz_preprocess.log 2>&1

if [ $? -ne 0 ]; then
    echo "Error: Preprocessing failed"
    echo "Check logs: /tmp/mpspdz_preprocess.log"
    exit 1
fi

# Parse preprocessing time
PREPROC_TIME=$(python3 << PYTHON_SCRIPT
import re
try:
    with open('/tmp/mpspdz_preprocess.log', 'r') as f:
        content = f.read()
    time_match = re.search(r'Time\s*=\s*([\d.]+)\s*seconds?', content, re.IGNORECASE)
    if time_match:
        print(time_match.group(1))
    else:
        print("0.0")
except:
    print("0.0")
PYTHON_SCRIPT
)

echo "Preprocessing time: ${PREPROC_TIME}s"

# Step 2: Switch to read mode for actual benchmarks
echo ""
echo "Switching to read mode for benchmark runs..."
toggle_mode "$PROG_MPC" "read"
./compile.py "$PROG_NAME" > /dev/null 2>&1

echo ""
echo "Starting benchmark runs..."
echo ""

for run in $(seq 1 $NUM_RUNS); do
    echo "=== Run $run/$NUM_RUNS ==="
    
    # Run MP-SPDZ with MASCOT protocol
    BENCH=1 Scripts/mascot.sh -v "$PROG_NAME" > /tmp/mpspdz_run${run}.log 2>&1
    
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
PREPROC_TIME = float("${PREPROC_TIME}")
PROG_NAME = "${PROG_NAME}"

def parse_logs():
    online_time = None
    comm_mb = None
    
    # Look for log files
    log_files = glob.glob(f'logs/{PROG_NAME}-mascot-party.x-N2-*')
    if not log_files:
        log_files = glob.glob(f'logs/{PROG_NAME}-*')
    
    # Try to find timing and communication info
    for log_file in sorted(log_files):
        try:
            with open(log_file, 'r') as f:
                content = f.read()
            
            # Look for detailed timing and communication breakdown
            # "X threads spent a total of Y seconds (Z MB, ...) on the online phase"
            # Example: 5 threads spent a total of 5.59104 seconds (0.108308 MB, 477 rounds) on the online phase
            stats_match = re.search(r'threads spent a total of ([\d.]+) seconds \(([\d.]+) MB', content, re.IGNORECASE)
            
            if stats_match:
                online_time = float(stats_match.group(1))
                comm_mb = float(stats_match.group(2))
                break
                
        except Exception as e:
            continue
    
    return online_time, comm_mb

online_time, comm_mb = parse_logs()

if online_time is not None and comm_mb is not None:
    print(f"{RUN_NUM},{PREPROC_TIME:.6f},{online_time:.6f},{comm_mb:.4f}")
else:
    # Fallback to tmp log
    try:
        with open(f'/tmp/mpspdz_run{RUN_NUM}.log', 'r') as f:
            content = f.read()
        
        stats_match = re.search(r'threads spent a total of ([\d.]+) seconds \(([\d.]+) MB', content, re.IGNORECASE)
        
        if stats_match:
            online_time = float(stats_match.group(1))
            comm_mb = float(stats_match.group(2))
            print(f"{RUN_NUM},{PREPROC_TIME:.6f},{online_time:.6f},{comm_mb:.4f}")
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
    fi
    sleep 1
done

echo ""
echo "Results saved to: $RESULTS_FILE"
echo "Done!"
