#!/bin/bash

# Script to run SEAL auction benchmark
# Usage: ./run_network_benchmark.sh <num_bidders> <domain_size> <num_runs>
# Note: SEAL doesn't use network RTT, so this just runs the benchmark multiple times

set -e

NUM_BIDDERS=${1:-100}
DOMAIN_SIZE=${2:-1000}
NUM_RUNS=${3:-3}

echo "=========================================="
echo "SEAL Auction Benchmark"
echo "=========================================="
echo "Number of bidders: $NUM_BIDDERS"
echo "Domain size: $DOMAIN_SIZE"
echo "Number of runs: $NUM_RUNS"
echo "=========================================="

# Change to SEAL directory
cd "$(dirname "$0")"

# Check if binary exists
if [ ! -f "build/bin/auction_benchmark" ]; then
    echo "Error: auction_benchmark binary not found. Please build first:"
    echo "  cd build && cmake .. && make"
    exit 1
fi

# Results directory
RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"
RESULTS_FILE="$RESULTS_DIR/seal_network_${NUM_BIDDERS}_${DOMAIN_SIZE}.csv"

# Write CSV header
echo "run,online_time_ms,comm_bytes_mb,comm_bytes_kb" > "$RESULTS_FILE"

echo ""
echo "Starting benchmark runs..."
echo ""

for run in $(seq 1 $NUM_RUNS); do
    echo "=== Run $run/$NUM_RUNS ==="
    
    # Run SEAL benchmark and capture output (with timeout)
    OUTPUT=$(timeout 300 ./build/bin/auction_benchmark $NUM_BIDDERS $DOMAIN_SIZE 2>&1)
    EXIT_CODE=$?
    
    if [ $EXIT_CODE -ne 0 ]; then
        echo "Error: Run $run failed (exit code: $EXIT_CODE)"
        echo "Output: $OUTPUT"
        continue
    fi
    
    # Parse results from output (skip header line, get data line)
    DATA_LINE=$(echo "$OUTPUT" | grep -v "^Bidders," | grep -v "^$" | tail -1)
    
    if [ -z "$DATA_LINE" ]; then
        echo "  Warning: No data line found in output"
        continue
    fi
    
    RESULT_LINE=$(echo "$DATA_LINE" | python3 << PYTHON_SCRIPT
import sys
import csv

RUN_NUM = $run

# Read from stdin (the CSV line from SEAL output)
line = sys.stdin.read().strip()
if not line or line.startswith('Bidders'):
    print(f"{RUN_NUM},ERROR,ERROR,ERROR", file=sys.stderr)
    sys.exit(1)

# Parse CSV line
fields = line.split(',')
if len(fields) < 11:
    print(f"{RUN_NUM},ERROR,ERROR,ERROR", file=sys.stderr)
    sys.exit(1)

try:
    # Fields: Bidders,BitWidth,MaxValue,ComputedMax,ActualMax,Status,ErrorPct,
    #         EncryptTime(ms),ComputeTime(ms),TotalTime(ms),CommSize(MB),...
    total_time_ms = float(fields[9])  # TotalTime(ms)
    comm_size_mb = float(fields[10])  # CommSize(MB)
    comm_bytes_kb = comm_size_mb * 1024.0
    
    print(f"{RUN_NUM},{total_time_ms:.3f},{comm_size_mb:.2f},{comm_bytes_kb:.2f}")
except (ValueError, IndexError) as e:
    print(f"{RUN_NUM},ERROR,ERROR,ERROR", file=sys.stderr)
PYTHON_SCRIPT
    )
    
    if [ -n "$RESULT_LINE" ] && [[ ! "$RESULT_LINE" =~ ERROR ]]; then
        echo "$RESULT_LINE" >> "$RESULTS_FILE"
        echo "  Result: $RESULT_LINE"
    else
        echo "  Warning: Failed to parse results for run $run"
        echo "  Output: $OUTPUT"
    fi
    
    # Small delay between runs
    sleep 0.5
done

echo ""
echo "Results saved to: $RESULTS_FILE"
echo "Done!"

