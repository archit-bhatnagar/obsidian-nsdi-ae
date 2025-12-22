#!/bin/bash

# Run network benchmarks with different configurations
# Usage: sudo ./run_network_tests.sh

set -e

cd "$(dirname "$0")"

# Test configurations: (num_clients, domain_size, rtt_ms, num_runs)
CONFIGS=(
    "25 1024 20 3"
    "50 1024 20 3"
    "100 1024 20 3"
    "25 1024 40 3"
    "50 1024 40 3"
)

echo "=========================================="
echo "Obsidian Network Benchmark - Multiple Configs"
echo "=========================================="
echo ""

for config in "${CONFIGS[@]}"; do
    read -r num_clients domain_size rtt_ms num_runs <<< "$config"
    echo ""
    echo ">>> Running: $num_clients clients, domain $domain_size, RTT ${rtt_ms}ms, $num_runs runs"
    echo ""
    
    ./run_network_benchmark.sh $num_clients $domain_size $rtt_ms $num_runs 2>&1 | grep -v "dump_bash_state"
    
    echo ""
    echo ">>> Completed: $num_clients clients, domain $domain_size, RTT ${rtt_ms}ms"
    echo ""
    sleep 2
done

echo ""
echo "=========================================="
echo "All tests completed!"
echo "=========================================="
echo ""
echo "Results saved in: results/network_benchmark_*.csv"

