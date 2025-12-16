#!/bin/bash

# Configuration for high throughput
LATENCY_FILE="latencies.txt"
RESULTS_DIR="results"
DURATION_SECONDS=60
MAX_CONCURRENT_PROCESSES=40  # Much higher than 40 cores
THROUGHPUT_LEVELS=(500 1000 2000 3000 4000 5000 6000 7000 8000)

# Create results directory
mkdir -p "$RESULTS_DIR"

# Build the binary in release mode
echo "Building binary in release mode..."
cargo build --release --bin dpf_tput
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

BINARY_PATH="./target/release/dpf_tput"

echo "Starting aggressive throughput benchmark..."
echo "Max concurrent processes: $MAX_CONCURRENT_PROCESSES"
echo "Test duration per level: ${DURATION_SECONDS} seconds"

# Function to run single auction and append result
run_auction_instance() {
    local output_file="$1"
    local temp_file=$(mktemp)
    
    # Clear the individual latency file
    > "latencies.txt"
    
    # Run auction
    timeout 30s "$BINARY_PATH" > /dev/null 2>&1
    
    # If successful, append latency to results
    if [ $? -eq 0 ] && [ -f "latencies.txt" ] && [ -s "latencies.txt" ]; then
        cat "latencies.txt" >> "$output_file"
    fi
    
    rm -f "$temp_file"
}

# Function to maintain target throughput with process saturation
run_saturated_benchmark() {
    local target_throughput=$1
    local test_duration=$2
    local output_file="$3"
    
    echo "Testing throughput: $target_throughput auctions/second"
    echo "  Saturating system with $MAX_CONCURRENT_PROCESSES concurrent processes"
    
    # Clear results file
    > "$output_file"
    
    # Calculate target auctions total
    local target_total=$((target_throughput * test_duration))
    
    local start_time=$(date +%s)
    local end_time=$((start_time + test_duration))
    local launched=0
    local completed=0
    
    # Launch initial burst of processes
    local pids=()
    
    # Function to launch auction and track completion
    launch_auction() {
        run_auction_instance "$output_file" &
        local pid=$!
        ((launched++))
        
        # Track completion in background
        {
            wait $pid 2>/dev/null
            echo "done" >> /tmp/completions_$$
        } &
    }
    
    # Create completion tracking file
    > /tmp/completions_$$
    
#     while (i < 100000) {
#     add_thread (all_computation);
#     sleep (1ms); // simulate throughput of 1000 auctions/sec
#     i++;
# } (e

    # Initial saturation - launch max concurrent processes
    echo "  Initial saturation: launching $MAX_CONCURRENT_PROCESSES processes..."
    for ((i=0; i<MAX_CONCURRENT_PROCESSES; i++)); do
        if [ $(date +%s) -ge $end_time ]; then
            break
        fi
        launch_auction
    done
    
    # Maintain saturation by launching new processes as others complete
    local last_check=0
    while [ $(date +%s) -lt $end_time ]; do
        sleep 0.1  # Brief check interval
        
        # Count completions
        if [ -f /tmp/completions_$$ ]; then
            local current_completed=$(wc -l < /tmp/completions_$$ 2>/dev/null || echo 0)
            local new_completions=$((current_completed - completed))
            completed=$current_completed
            
            # Launch new processes to replace completed ones
            for ((i=0; i<new_completions; i++)); do
                if [ $(date +%s) -ge $end_time ]; then
                    break
                fi
                launch_auction
            done
        fi
        
        # Progress update every 5 seconds
        local current_time=$(date +%s)
        if [ $((current_time - last_check)) -ge 5 ]; then
            local actual_completed=$(wc -l < "$output_file" 2>/dev/null || echo 0)
            local elapsed=$((current_time - start_time))
            local current_rate=$(echo "scale=1; $actual_completed / $elapsed" | bc -l 2>/dev/null || echo "0")
            echo "    ${elapsed}s: $actual_completed auctions completed ($current_rate aps, $launched launched)"
            last_check=$current_time
        fi
    done
    
    echo "  Waiting for remaining processes to complete..."
    wait  # Wait for all background processes
    
    # Final count
    local final_completed=$(wc -l < "$output_file" 2>/dev/null || echo 0)
    local actual_throughput=$(echo "scale=2; $final_completed / $test_duration" | bc -l)
    echo "  Final: $final_completed auctions completed"
    echo "  Actual throughput: $actual_throughput auctions/second"
    echo "  Efficiency: $(echo "scale=1; $actual_throughput * 100 / $target_throughput" | bc -l)%"
    
    # Cleanup
    rm -f /tmp/completions_$$
    
    return $final_completed
}

# Alternative: Burst-based approach for maximum throughput
run_burst_benchmark() {
    local target_throughput=$1
    local test_duration=$2
    local output_file="$3"
    
    echo "Testing BURST throughput: $target_throughput auctions/second"
    
    > "$output_file"
    
    local burst_size=200  # Launch 200 at a time
    local interval=$(echo "scale=6; $burst_size / $target_throughput" | bc -l)
    
    local start_time=$(date +%s)
    local end_time=$((start_time + test_duration))
    local total_launched=0
    
    while [ $(date +%s) -lt $end_time ]; do
        local burst_start=$(date +%s.%N)
        
        # Launch burst of processes
        local pids=()
        for ((i=0; i<burst_size; i++)); do
            if [ $(date +%s) -ge $end_time ]; then
                break
            fi
            
            run_auction_instance "$output_file" &
            pids+=($!)
            ((total_launched++))
        done
        
        # Wait for burst to complete
        for pid in "${pids[@]}"; do
            wait "$pid" 2>/dev/null
        done
        
        # Calculate sleep time for next burst
        local burst_elapsed=$(echo "$(date +%s.%N) - $burst_start" | bc -l)
        local sleep_time=$(echo "$interval - $burst_elapsed" | bc -l)
        
        if (( $(echo "$sleep_time > 0" | bc -l) )); then
            sleep "$sleep_time"
        fi
        
        # Progress update
        local completed=$(wc -l < "$output_file" 2>/dev/null || echo 0)
        local elapsed=$(($(date +%s) - start_time))
        if [ $((elapsed % 10)) -eq 0 ] && [ $elapsed -gt 0 ]; then
            local rate=$(echo "scale=1; $completed / $elapsed" | bc -l)
            echo "    ${elapsed}s: $completed completed, $total_launched launched ($rate aps)"
        fi
    done
    
    local final_completed=$(wc -l < "$output_file" 2>/dev/null || echo 0)
    echo "  Burst result: $final_completed auctions completed"
    echo "  Actual throughput: $(echo "scale=2; $final_completed / $test_duration" | bc -l) aps"
}

# Monitor system resources
monitor_system() {
    local duration=$1
    local output_file="system_monitor.log"
    
    > "$output_file"
    local end_time=$(($(date +%s) + duration))
    
    while [ $(date +%s) -lt $end_time ]; do
        local cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//')
        local mem_usage=$(free | grep Mem | awk '{printf "%.1f", $3/$2 * 100.0}')
        local process_count=$(pgrep -cf "dpf_tput" || echo 0)
        
        echo "$(date '+%H:%M:%S') CPU:${cpu_usage}% MEM:${mem_usage}% PROC:${process_count}" >> "$output_file"
        sleep 2
    done
}

# Run benchmarks
for throughput in "${THROUGHPUT_LEVELS[@]}"; do
    echo ""
    echo "=================================================="
    
    output_file="$RESULTS_DIR/latencies_${throughput}_aps.txt"
    
    # Start system monitoring
    monitor_system $DURATION_SECONDS &
    monitor_pid=$!
    
    # Choose strategy based on throughput level
    if [ $throughput -le 2000 ]; then
        run_saturated_benchmark "$throughput" "$DURATION_SECONDS" "$output_file"
    else
        run_burst_benchmark "$throughput" "$DURATION_SECONDS" "$output_file"
    fi
    
    # Stop monitoring
    kill $monitor_pid 2>/dev/null
    wait $monitor_pid 2>/dev/null
    
    # Kill any remaining processes
    pkill -f "dpf_tput" 2>/dev/null
    sleep 2
    
    echo "  ✓ Test completed"
    echo ""
done

echo "All benchmarks completed! Check $RESULTS_DIR/ for results."

# Generate quick analysis
cat > "$RESULTS_DIR/quick_analysis.py" << 'EOF'
import os
import numpy as np

print("QUICK THROUGHPUT ANALYSIS")
print("=" * 50)

for filename in sorted(os.listdir('.')):
    if filename.startswith('latencies_') and filename.endswith('_aps.txt'):
        throughput = filename.replace('latencies_', '').replace('_aps.txt', '')
        
        try:
            latencies = np.loadtxt(filename)
            if len(latencies) > 0:
                actual_throughput = len(latencies) / 60  # 60 second test
                p50 = np.percentile(latencies, 50)
                p99 = np.percentile(latencies, 99)
                print(f"{throughput:>4s} aps target: {actual_throughput:6.1f} actual, P50: {p50:6.1f}ms, P99: {p99:6.1f}ms, samples: {len(latencies)}")
        except:
            print(f"{throughput:>4s} aps target: FAILED")

EOF

echo "Run 'cd $RESULTS_DIR && python quick_analysis.py' for quick results"
