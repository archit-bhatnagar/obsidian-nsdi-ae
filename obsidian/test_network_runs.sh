#!/bin/bash

# Quick test script to run a few Obsidian network benchmark instances
# Usage: sudo ./test_network_runs.sh

set -e

echo "=========================================="
echo "Obsidian Network Benchmark - Test Runs"
echo "=========================================="
echo ""

# Test configurations: (num_clients, domain_size, rtt_ms, num_runs)
TEST_CONFIGS=(
    "25 1024 20 2"
    "25 1024 40 2"
    "50 1024 20 2"
)

cd "$(dirname "$0")"

for config in "${TEST_CONFIGS[@]}"; do
    read -r num_clients domain_size rtt_ms num_runs <<< "$config"
    echo ""
    echo ">>> Running: $num_clients clients, domain $domain_size, RTT ${rtt_ms}ms, $num_runs runs"
    echo ""
    
    ./run_network_benchmark.sh $num_clients $domain_size $rtt_ms $num_runs
    
    echo ""
    echo ">>> Completed: $num_clients clients, domain $domain_size, RTT ${rtt_ms}ms"
    echo ""
    sleep 2
done

echo ""
echo "=========================================="
echo "All test runs completed!"
echo "=========================================="

