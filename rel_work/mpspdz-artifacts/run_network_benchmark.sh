#!/bin/bash

# Script to run MP-SPDZ Vickrey auction with netem RTT emulation
# Usage: sudo ./run_network_benchmark.sh <num_bidders> <domain_size> <rtt_ms> <num_runs>
# Note: domain_size determines the number of bid shares (100, 1000, 10000)

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

# Generate input files for the domain size
echo ""
echo "Generating input files for domain size $DOMAIN_SIZE..."
python3 generate_inputs.py $DOMAIN_SIZE Player-Data

# Program name based on domain size (e.g., vickrey100, vickrey1000, vickrey10000)
PROG_NAME="vickrey${DOMAIN_SIZE}"
VICKREY_MPC="Programs/Source/vickrey.mpc"

# Check if source file exists
if [ ! -f "$VICKREY_MPC" ]; then
    echo "Error: $VICKREY_MPC not found"
    exit 1
fi

# Backup original file
cp "$VICKREY_MPC" "${VICKREY_MPC}.bak"

# Function to toggle write/read mode in vickrey.mpc
toggle_mode() {
    local mode=$1  # "write" or "read"
    
    if [ "$mode" == "write" ]; then
        # Uncomment write, comment read
        sed -i 's/^# input.write_to_file()/input.write_to_file()/' "$VICKREY_MPC"
        sed -i 's/^ret = bids_from_file.read_from_file(start= 0)/# ret = bids_from_file.read_from_file(start= 0)/' "$VICKREY_MPC"
    else
        # Comment write, uncomment read
        sed -i 's/^input.write_to_file()/# input.write_to_file()/' "$VICKREY_MPC"
        sed -i 's/^# ret = bids_from_file.read_from_file(start= 0)/ret = bids_from_file.read_from_file(start= 0)/' "$VICKREY_MPC"
    fi
}

# Compile program with domain-specific name
echo ""
echo "Compiling program as $PROG_NAME..."
cp "$VICKREY_MPC" "Programs/Source/${PROG_NAME}.mpc"
./compile.py "$PROG_NAME" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo "Error: Failed to compile $PROG_NAME"
    mv "${VICKREY_MPC}.bak" "$VICKREY_MPC"
    exit 1
fi

# Check if compiled binary exists
if [ ! -f "mascot-party.x" ]; then
    echo "Error: mascot-party.x not found. Please build MP-SPDZ first"
    mv "${VICKREY_MPC}.bak" "$VICKREY_MPC"
    exit 1
fi

# Results directory
RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"
RESULTS_FILE="$RESULTS_DIR/mpspdz_network_${NUM_BIDDERS}_${DOMAIN_SIZE}_${RTT_MS}ms.csv"

# Write CSV header
echo "run,preprocessing_time_s,online_time_s,comm_bytes_mb,comm_bytes_kb" > "$RESULTS_FILE"

echo ""
echo "Generating persistence shares (preprocessing phase)..."
echo ""

# Step 1: Generate persistence shares (write mode)
toggle_mode "write"
cp "$VICKREY_MPC" "Programs/Source/${PROG_NAME}.mpc"
./compile.py "$PROG_NAME" > /dev/null 2>&1

# Clear persistence files first
./clear_persist.sh 2>/dev/null || true

# Run preprocessing phase (generates persistence)
echo "Running preprocessing to generate persistence shares..."
BENCH=1 Scripts/mascot.sh -v "$PROG_NAME" > /tmp/mpspdz_preprocess.log 2>&1

if [ $? -ne 0 ]; then
    echo "Error: Preprocessing failed"
    echo "Check logs: /tmp/mpspdz_preprocess.log"
    toggle_mode "read"
    mv "${VICKREY_MPC}.bak" "$VICKREY_MPC"
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
toggle_mode "read"
cp "$VICKREY_MPC" "Programs/Source/${PROG_NAME}.mpc"
./compile.py "$PROG_NAME" > /dev/null 2>&1

echo ""
echo "Starting benchmark runs..."
echo ""

for run in $(seq 1 $NUM_RUNS); do
    echo "=== Run $run/$NUM_RUNS ==="
    
    # Don't clear persistence files - we want to reuse them
    # ./clear_persist.sh 2>/dev/null || true
    
    # Run MP-SPDZ with MASCOT protocol (read mode, uses persistence)
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
    comm_bytes = None
    
    # Look for log files
    log_files = glob.glob(f'logs/{PROG_NAME}-mascot-party.x-N2-*')
    if not log_files:
        log_files = glob.glob(f'logs/{PROG_NAME}-*')
    
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
    print(f"{RUN_NUM},{PREPROC_TIME:.6f},{online_time:.6f},{comm_mb:.2f},{comm_kb:.2f}")
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
            print(f"{RUN_NUM},{PREPROC_TIME:.6f},{online_time:.6f},{comm_mb:.2f},{comm_kb:.2f}")
        else:
            print(f"{RUN_NUM},ERROR,ERROR,ERROR,ERROR", file=sys.stderr)
    except:
        print(f"{RUN_NUM},ERROR,ERROR,ERROR,ERROR", file=sys.stderr)
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

# Restore original file
mv "${VICKREY_MPC}.bak" "$VICKREY_MPC"

echo ""
echo "Results saved to: $RESULTS_FILE"
echo "Done!"
