#!/bin/bash

# Configuration
BINARY="./target/release/dpf_tput"
TEST_DURATION=10  # seconds to test each throughput level
THROUGHPUT_LEVELS=(1 2 5 10 20 30 50 75 100 150 200 300 500)
RESULTS_FILE="throughput_results.csv"

# Build the binary
echo "Building binary..."
cargo build --release --bin dpf_tput
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

# Initialize results file
echo "target_aps,actual_aps,samples,p50_ms,p99_ms,mean_ms,min_ms,max_ms" > "$RESULTS_FILE"

echo "Starting single-core throughput benchmark..."
echo "Test duration per level: ${TEST_DURATION} seconds"
echo "Target throughput levels: ${THROUGHPUT_LEVELS[*]}"
echo ""

# Function to calculate percentiles from latencies file
calculate_stats() {
    local latency_file="$1"
    
    if [ ! -f "$latency_file" ] || [ ! -s "$latency_file" ]; then
        echo "0,0,0,0,0,0"
        return
    fi
    
    # Sort latencies and calculate stats
    local temp_sorted=$(mktemp)
    sort -n "$latency_file" > "$temp_sorted"
    
    local count=$(wc -l < "$temp_sorted")
    if [ $count -eq 0 ]; then
        echo "0,0,0,0,0,0"
        rm "$temp_sorted"
        return
    fi
    
    # Calculate percentile indices
    local p50_idx=$(( (count * 50 + 50) / 100 ))
    local p99_idx=$(( (count * 99 + 50) / 100 ))
    
    # Ensure indices are at least 1
    [ $p50_idx -lt 1 ] && p50_idx=1
    [ $p99_idx -lt 1 ] && p99_idx=1
    
    # Get percentile values
    local p50=$(sed -n "${p50_idx}p" "$temp_sorted")
    local p99=$(sed -n "${p99_idx}p" "$temp_sorted")
    local min=$(head -n1 "$temp_sorted")
    local max=$(tail -n1 "$temp_sorted")
    
    # Calculate mean
    local mean=$(awk '{sum += $1} END {print (NR > 0) ? sum/NR : 0}' "$temp_sorted")
    
    echo "$count,$p50,$p99,$mean,$min,$max"
    rm "$temp_sorted"
}

# Function to run benchmark at specific throughput
run_throughput_test() {
    local target_aps=$1
    echo "Testing $target_aps auctions/second..."
    
    # Clear previous results
    rm -f latencies.txt
    
    # Calculate interval between auction starts (in seconds)
    local interval=$(echo "scale=6; 1.0 / $target_aps" | bc -l)
    
    local start_time=$(date +%s)
    local end_time=$((start_time + TEST_DURATION))
    local launched=0
    local pids=()
    
    echo "  Interval: ${interval}s, Duration: ${TEST_DURATION}s"
    
    # Launch auctions at target rate
    while [ $(date +%s) -lt $end_time ]; do
        local batch_start=$(date +%s.%N)
        
        # Launch auction in background
        taskset -c 0 "$BINARY" > /dev/null 2>&1 &
        local pid=$!
        pids+=($pid)
        ((launched++))
        
        # Clean up completed processes periodically
        if [ ${#pids[@]} -gt 100 ]; then
            local new_pids=()
            for pid in "${pids[@]}"; do
                if kill -0 "$pid" 2>/dev/null; then
                    new_pids+=($pid)
                fi
            done
            pids=("${new_pids[@]}")
        fi
        
        # Progress indicator
        if [ $((launched % 50)) -eq 0 ]; then
            local elapsed=$(($(date +%s) - start_time))
            echo "    Launched: $launched auctions (${elapsed}s elapsed)"
        fi
        
        # Wait for next auction interval
        local batch_elapsed=$(echo "$(date +%s.%N) - $batch_start" | bc -l)
        local sleep_time=$(echo "$interval - $batch_elapsed" | bc -l)
        
        if (( $(echo "$sleep_time > 0.001" | bc -l) )); then
            sleep "$sleep_time"
        fi
    done
    
    echo "  Waiting for remaining auctions to complete..."
    
    # Wait for all background processes with timeout
    local wait_start=$(date +%s)
    for pid in "${pids[@]}"; do
        # Don't wait more than 30 seconds total
        if [ $(($(date +%s) - wait_start)) -gt 30 ]; then
            kill "$pid" 2>/dev/null
        else
            wait "$pid" 2>/dev/null
        fi
    done
    
    # Calculate actual throughput and stats
    local actual_aps=0
    local stats="0,0,0,0,0,0"
    
    if [ -f "latencies.txt" ]; then
        stats=$(calculate_stats "latencies.txt")
        local completed=$(echo "$stats" | cut -d',' -f1)
        actual_aps=$(echo "scale=2; $completed / $TEST_DURATION" | bc -l)
    fi
    
    # Extract individual stats
    local samples=$(echo "$stats" | cut -d',' -f1)
    local p50=$(echo "$stats" | cut -d',' -f2)
    local p99=$(echo "$stats" | cut -d',' -f3)
    local mean=$(echo "$stats" | cut -d',' -f4)
    local min=$(echo "$stats" | cut -d',' -f5)
    local max=$(echo "$stats" | cut -d',' -f6)
    
    # Save results
    echo "$target_aps,$actual_aps,$samples,$p50,$p99,$mean,$min,$max" >> "$RESULTS_FILE"
    
    # Display results
    printf "  Results: %.1f actual aps, %d samples, P50=%.2fms, P99=%.2fms\n" \
           "$actual_aps" "$samples" "$p50" "$p99"
    
    # Efficiency calculation
    if (( $(echo "$target_aps > 0" | bc -l) )); then
        local efficiency=$(echo "scale=1; $actual_aps * 100 / $target_aps" | bc -l)
        printf "  Efficiency: %.1f%%\n" "$efficiency"
    fi
    
    echo ""
}

# Main benchmark loop
echo "=========================================="
echo "SINGLE-CORE AUCTION THROUGHPUT BENCHMARK"
echo "=========================================="

for target_aps in "${THROUGHPUT_LEVELS[@]}"; do
    run_throughput_test "$target_aps"
    
    # Brief pause between tests
    sleep 2
done

# Generate summary report
echo "=========================================="
echo "BENCHMARK SUMMARY"
echo "=========================================="
echo ""

# Display results table
printf "%-10s %-12s %-8s %-10s %-10s %-10s %-12s\n" \
       "Target" "Actual" "Samples" "P50 (ms)" "P99 (ms)" "Mean (ms)" "Efficiency"
printf "%-10s %-12s %-8s %-10s %-10s %-10s %-12s\n" \
       "------" "------" "-------" "---------" "---------" "---------" "----------"

# Read and display results
tail -n +2 "$RESULTS_FILE" | while IFS=',' read -r target actual samples p50 p99 mean min max; do
    local efficiency=""
    if (( $(echo "$target > 0" | bc -l) )); then
        efficiency=$(echo "scale=1; $actual * 100 / $target" | bc -l)
        efficiency="${efficiency}%"
    fi
    
    printf "%-10.0f %-12.1f %-8d %-10.2f %-10.2f %-10.2f %-12s\n" \
           "$target" "$actual" "$samples" "$p50" "$p99" "$mean" "$efficiency"
done

echo ""
echo "Detailed results saved to: $RESULTS_FILE"

# Generate Python analysis script
cat > "analyze_results.py" << 'EOF'
#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

# Read results
df = pd.read_csv('throughput_results.csv')

# Create plots
fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(15, 12))

# Latency vs Throughput
ax1.plot(df['actual_aps'], df['p50_ms'], 'o-', label='P50', linewidth=2)
ax1.plot(df['actual_aps'], df['p99_ms'], 's-', label='P99', linewidth=2)
ax1.set_xlabel('Actual Throughput (auctions/sec)')
ax1.set_ylabel('Latency (ms)')
ax1.set_title('Latency vs Achieved Throughput')
ax1.legend()
ax1.grid(True, alpha=0.3)
ax1.set_yscale('log')

# Efficiency
efficiency = df['actual_aps'] / df['target_aps'] * 100
ax2.plot(df['target_aps'], efficiency, 'go-', linewidth=2)
ax2.axhline(y=100, color='r', linestyle='--', alpha=0.5, label='100% efficiency')
ax2.set_xlabel('Target Throughput (auctions/sec)')
ax2.set_ylabel('Efficiency (%)')
ax2.set_title('System Efficiency')
ax2.legend()
ax2.grid(True, alpha=0.3)

# Actual vs Target throughput
ax3.plot([0, df['target_aps'].max()], [0, df['target_aps'].max()], 'k--', alpha=0.5, label='Perfect scaling')
ax3.plot(df['target_aps'], df['actual_aps'], 'ro-', linewidth=2, label='Actual')
ax3.set_xlabel('Target Throughput (auctions/sec)')
ax3.set_ylabel('Actual Throughput (auctions/sec)')
ax3.set_title('Throughput Scaling')
ax3.legend()
ax3.grid(True, alpha=0.3)

# Sample count
ax4.bar(df['target_aps'], df['samples'], alpha=0.7)
ax4.set_xlabel('Target Throughput (auctions/sec)')
ax4.set_ylabel('Completed Auctions')
ax4.set_title('Sample Count per Test')
ax4.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig('single_core_analysis.png', dpi=300, bbox_inches='tight')
print("Analysis plots saved to: single_core_analysis.png")

# Find optimal operating point
print("\nKEY FINDINGS:")
print("-" * 40)

# Max efficient throughput (>90% efficiency)
efficient = df[efficiency >= 90]
if len(efficient) > 0:
    max_efficient = efficient['actual_aps'].max()
    print(f"Max efficient throughput (≥90%): {max_efficient:.1f} aps")

# Max throughput with reasonable latency (P99 < 100ms)
reasonable = df[df['p99_ms'] < 100]
if len(reasonable) > 0:
    max_reasonable = reasonable['actual_aps'].max()
    print(f"Max throughput with P99 < 100ms: {max_reasonable:.1f} aps")

# Overall max
print(f"Peak throughput achieved: {df['actual_aps'].max():.1f} aps")
print(f"Best P50 latency: {df['p50_ms'].min():.1f}ms")
print(f"Best P99 latency: {df['p99_ms'].min():.1f}ms")
EOF

chmod +x analyze_results.py

echo ""
echo "To generate analysis plots, run:"
echo "pip install pandas matplotlib numpy"
echo "python3 analyze_results.py"
