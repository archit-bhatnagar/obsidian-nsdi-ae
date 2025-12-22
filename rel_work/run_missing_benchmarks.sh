#!/bin/bash

# Script to run only missing benchmark configurations
# Usage: sudo ./run_missing_benchmarks.sh [num_runs]

set -e

NUM_RUNS=${1:-3}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Running Missing Network Benchmarks"
echo "=========================================="
echo "Number of runs per configuration: $NUM_RUNS"
echo ""

# Expected configurations
CONFIGS=(
    "25 100 20"
    "25 100 40"
    "25 1000 20"
    "25 1000 40"
    "25 10000 20"
    "25 10000 40"
    "50 100 20"
    "50 100 40"
    "50 1000 20"
    "50 1000 40"
    "50 10000 20"
    "50 10000 40"
    "100 100 20"
    "100 100 40"
    "100 1000 20"
    "100 1000 40"
    "100 10000 20"
    "100 10000 40"
)

# Check and run missing Obsidian benchmarks
echo ">>> Checking Obsidian benchmarks..."
cd "$SCRIPT_DIR/obsidian"
OBSIDIAN_COUNT=0
for config in "${CONFIGS[@]}"; do
    read -r num_bidders domain_size rtt_ms <<< "$config"
    CSV_FILE="results/network_benchmark_${num_bidders}_${domain_size}_${rtt_ms}ms.csv"
    if [ ! -f "$CSV_FILE" ]; then
        echo "  Running Obsidian: $num_bidders bidders, domain $domain_size, RTT ${rtt_ms}ms"
        sudo bash ./run_network_benchmark.sh $num_bidders $domain_size $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -3
        OBSIDIAN_COUNT=$((OBSIDIAN_COUNT + 1))
    fi
done
echo "  Obsidian: Ran $OBSIDIAN_COUNT missing configurations"
echo ""

# Check and run missing Addax benchmarks
echo ">>> Checking Addax benchmarks..."
cd "$SCRIPT_DIR/addax"
ADDAX_COUNT=0
for config in "${CONFIGS[@]}"; do
    read -r num_bidders bucket_num rtt_ms <<< "$config"
    CSV_FILE="auction/auction-local-computation/results/addax_network_${num_bidders}_${bucket_num}_${rtt_ms}ms.csv"
    if [ ! -f "$CSV_FILE" ]; then
        echo "  Running Addax: $num_bidders bidders, $bucket_num buckets, RTT ${rtt_ms}ms"
        sudo bash ./run_network_benchmark.sh $num_bidders $bucket_num $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -3
        ADDAX_COUNT=$((ADDAX_COUNT + 1))
    fi
done
echo "  Addax: Ran $ADDAX_COUNT missing configurations"
echo ""

# Check and run missing MP-SPDZ benchmarks
echo ">>> Checking MP-SPDZ benchmarks..."
cd "$SCRIPT_DIR/mp-spdz-0.3.9"
MPSPDZ_COUNT=0
for config in "${CONFIGS[@]}"; do
    read -r num_bidders domain_size rtt_ms <<< "$config"
    CSV_FILE="results/mpspdz_network_${num_bidders}_${rtt_ms}ms.csv"
    if [ ! -f "$CSV_FILE" ]; then
        echo "  Running MP-SPDZ: $num_bidders bidders, RTT ${rtt_ms}ms"
        sudo bash ./run_network_benchmark.sh $num_bidders $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -3
        MPSPDZ_COUNT=$((MPSPDZ_COUNT + 1))
    fi
done
echo "  MP-SPDZ: Ran $MPSPDZ_COUNT missing configurations"
echo ""

echo "=========================================="
echo "Completed running missing benchmarks!"
echo "=========================================="
echo ""
echo "To regenerate plots, run:"
echo "  python3 plot_network_benchmarks.py"

