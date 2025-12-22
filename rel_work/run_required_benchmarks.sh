#!/bin/bash

# Script to run only the required benchmark configurations for the plots
# - Varying bidders: domain fixed at 1000, bidders: 25, 50, 100, RTT: 20ms, 40ms
# - Varying domain: bidders fixed at 100, domains: 100, 1000, 10000, RTT: 20ms, 40ms

set -e

NUM_RUNS=${1:-3}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Running Required Network Benchmarks"
echo "=========================================="
echo "Number of runs per configuration: $NUM_RUNS"
echo ""

# Required configurations for plots
# Varying bidders (domain=1000): bidders 25, 50, 100 with RTT 20ms, 40ms
# Varying domain (bidders=100): domains 100, 1000, 10000 with RTT 20ms, 40ms
REQUIRED_CONFIGS=(
    "25 1000 20"
    "25 1000 40"
    "50 1000 20"
    "50 1000 40"
    "100 1000 20"
    "100 1000 40"
    "100 100 20"
    "100 100 40"
    "100 10000 20"
    "100 10000 40"
)

echo ">>> Running Obsidian benchmarks..."
cd "$SCRIPT_DIR/../obsidian"
for config in "${REQUIRED_CONFIGS[@]}"; do
    read -r num_bidders domain_size rtt_ms <<< "$config"
    CSV_FILE="results/network_benchmark_${num_bidders}_${domain_size}_${rtt_ms}ms.csv"
    if [ ! -f "$CSV_FILE" ]; then
        echo "  Running: $num_bidders bidders, domain $domain_size, RTT ${rtt_ms}ms"
        sudo bash ./run_network_benchmark.sh $num_bidders $domain_size $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -3
    else
        echo "  Skipping (exists): $num_bidders bidders, domain $domain_size, RTT ${rtt_ms}ms"
    fi
done

echo ""
echo ">>> Running Addax benchmarks..."
cd "$SCRIPT_DIR/addax"
for config in "${REQUIRED_CONFIGS[@]}"; do
    read -r num_bidders bucket_num rtt_ms <<< "$config"
    CSV_FILE="auction/auction-local-computation/results/addax_network_${num_bidders}_${bucket_num}_${rtt_ms}ms.csv"
    if [ ! -f "$CSV_FILE" ]; then
        echo "  Running: $num_bidders bidders, $bucket_num buckets, RTT ${rtt_ms}ms"
        sudo bash ./run_network_benchmark.sh $num_bidders $bucket_num $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -3
    else
        echo "  Skipping (exists): $num_bidders bidders, $bucket_num buckets, RTT ${rtt_ms}ms"
    fi
done

echo ""
echo ">>> Running MP-SPDZ benchmarks..."
cd "$SCRIPT_DIR/mp-spdz-0.3.9"
for config in "${REQUIRED_CONFIGS[@]}"; do
    read -r num_bidders domain_size rtt_ms <<< "$config"
    CSV_FILE="results/mpspdz_network_${num_bidders}_${rtt_ms}ms.csv"
    if [ ! -f "$CSV_FILE" ]; then
        echo "  Running: $num_bidders bidders, RTT ${rtt_ms}ms"
        sudo bash ./run_network_benchmark.sh $num_bidders $rtt_ms $NUM_RUNS 2>&1 | grep -v "dump_bash_state" | tail -3
    else
        echo "  Skipping (exists): $num_bidders bidders, RTT ${rtt_ms}ms"
    fi
done

echo ""
echo "=========================================="
echo "Completed running required benchmarks!"
echo "=========================================="
echo ""
echo "To regenerate plots, run:"
echo "  python3 plot_network_benchmarks.py"

