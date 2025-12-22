#!/bin/bash

# Script to run all MP-SPDZ network benchmark configurations
# This script should be run from the MP-SPDZ installation directory (mp-spdz-0.3.9)
# Usage: sudo ./run_all_benchmarks.sh [num_runs]

NUM_RUNS=${1:-3}

echo "=========================================="
echo "Running All MP-SPDZ Network Benchmarks"
echo "=========================================="
echo "Number of runs per configuration: $NUM_RUNS"
echo ""

# Configurations: (num_bidders, rtt_ms)
CONFIGS=(
    "100 20"
    "100 40"
    "100 60"
    "50 20"
    "50 40"
    "50 60"
    "25 20"
    "25 40"
    "25 60"
)

for config in "${CONFIGS[@]}"; do
    read -r num_bidders rtt_ms <<< "$config"
    echo ""
    echo ">>> Running: $num_bidders bidders, ${rtt_ms}ms RTT"
    echo ""
    
    ./run_network_benchmark.sh $num_bidders $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state"
    
    echo ""
    echo ">>> Completed: $num_bidders bidders, ${rtt_ms}ms RTT"
    echo ""
    sleep 2
done

echo ""
echo "=========================================="
echo "All benchmarks completed!"
echo "=========================================="
echo ""
echo "Results saved in: results/"

