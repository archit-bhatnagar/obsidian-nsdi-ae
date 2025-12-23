#!/bin/bash

# Script to run all Addax network benchmark configurations
# Usage: sudo ./run_all_benchmarks.sh [num_runs]

NUM_RUNS=${1:-3}

echo "=========================================="
echo "Running All Addax Network Benchmarks"
echo "=========================================="
echo "Number of runs per configuration: $NUM_RUNS"
echo ""

# Configurations: (num_bidders, bucket_num, rtt_ms)
CONFIGS=(
    "100 10000 20"
    "100 10000 40"
    "100 10000 60"
    "100 1000 20"
    "100 1000 40"
    "100 1000 60"
    "50 1000 20"
    "50 1000 40"
    "50 1000 60"
    "25 1000 20"
    "25 1000 40"
    "25 1000 60"
    "100 100 20"
    "100 100 40"
    "100 100 60"
)

cd "$(dirname "$0")"

for config in "${CONFIGS[@]}"; do
    read -r num_bidders bucket_num rtt_ms <<< "$config"
    echo ""
    echo ">>> Running: $num_bidders bidders, $bucket_num buckets, ${rtt_ms}ms RTT"
    echo ""
    
    ./run_network_benchmark.sh $num_bidders $bucket_num $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state"
    
    echo ""
    echo ">>> Completed: $num_bidders bidders, $bucket_num buckets, ${rtt_ms}ms RTT"
    echo ""
    sleep 2
done

echo ""
echo "=========================================="
echo "All benchmarks completed!"
echo "=========================================="
echo ""
echo "Results saved in: auction/auction-local-computation/results/"

