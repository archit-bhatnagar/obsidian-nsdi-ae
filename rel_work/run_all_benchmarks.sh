#!/bin/bash

# Master script to run all benchmarks (Obsidian, Addax, MP-SPDZ, SEAL)
# Usage: sudo ./run_all_benchmarks.sh [num_runs]

set -e

NUM_RUNS=${1:-3}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Running All Network Benchmarks"
echo "=========================================="
echo "Number of runs per configuration: $NUM_RUNS"
echo ""

# Configurations: (num_bidders, domain_size/bucket_num, rtt_ms)
# For SEAL, domain_size is used; for others, bucket_num/domain_size
CONFIGS=(
    "100 10000 20"
    "100 10000 40"
    "100 1000 20"
    "100 1000 40"
    "100 100 20"
    "100 100 40"
    "50 1000 20"
    "50 1000 40"
    "25 1000 20"
    "25 1000 40"
)

echo ">>> Running Obsidian benchmarks..."
cd "$SCRIPT_DIR/obsidian"
for config in "${CONFIGS[@]}"; do
    read -r num_bidders domain_size rtt_ms <<< "$config"
    echo "  Obsidian: $num_bidders bidders, domain $domain_size, RTT ${rtt_ms}ms"
    sudo bash ./run_network_benchmark.sh $num_bidders $domain_size $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -5
done

echo ""
echo ">>> Running Addax benchmarks..."
cd "$SCRIPT_DIR/addax"
for config in "${CONFIGS[@]}"; do
    read -r num_bidders bucket_num rtt_ms <<< "$config"
    echo "  Addax: $num_bidders bidders, $bucket_num buckets, RTT ${rtt_ms}ms"
    sudo bash ./run_network_benchmark.sh $num_bidders $bucket_num $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -5
done

echo ""
echo ">>> Running MP-SPDZ benchmarks..."
cd "$SCRIPT_DIR/mp-spdz-0.3.9"
for config in "${CONFIGS[@]}"; do
    read -r num_bidders domain_size rtt_ms <<< "$config"
    echo "  MP-SPDZ: $num_bidders bidders, RTT ${rtt_ms}ms"
    sudo bash ./run_network_benchmark.sh $num_bidders $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -5
done

# SEAL benchmarks skipped for now (not working as intended)
# echo ""
# echo ">>> Running SEAL benchmarks..."
# cd "$SCRIPT_DIR/seal-auction"
# for config in "${CONFIGS[@]}"; do
#     read -r num_bidders domain_size rtt_ms <<< "$config"
#     # SEAL doesn't use RTT, so we just run with domain_size
#     echo "  SEAL: $num_bidders bidders, domain $domain_size"
#     bash ./run_network_benchmark.sh $num_bidders $domain_size $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -5
# done

echo ""
echo "=========================================="
echo "All benchmarks completed!"
echo "=========================================="
echo ""
echo "Results saved in:"
echo "  - obsidian/results/"
echo "  - addax/auction/auction-local-computation/results/"
echo "  - mp-spdz-0.3.9/results/"
echo ""
echo "To generate plots, run:"
echo "  python3 plot_network_benchmarks.py"

