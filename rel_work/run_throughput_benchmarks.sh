#!/bin/bash

# Script to run throughput benchmarks for Obsidian and Addax
# Usage: sudo ./run_throughput_benchmarks.sh

# Don't exit on error for Obsidian section
set +e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Running Throughput Benchmarks"
echo "=========================================="
echo ""

# ============================================
# Obsidian Throughput
# ============================================
echo ">>> Running Obsidian throughput benchmarks..."
cd "$SCRIPT_DIR/../obsidian"

# Build if needed
if [ ! -f "./target/release/dpf_tput" ]; then
    echo "  Building dpf_tput..."
    export RUSTFLAGS+="-C target-cpu=native"
    cargo build --release --bin dpf_tput 2>&1 || {
        echo "  Warning: Obsidian build failed. Skipping Obsidian benchmarks."
        echo "  (You can run Obsidian benchmarks separately later)"
        echo ""
    }
fi

if [ -f "./target/release/dpf_tput" ]; then
    export RUSTFLAGS+="-C target-cpu=native"
    # Run Obsidian throughput benchmark
    echo "  Running Obsidian throughput test..."
    bash ./bench_tput.sh
    echo "  Obsidian results saved to: throughput_results.csv"
else
    echo "  Skipping Obsidian (binary not available)"
fi
echo ""

# Enable exit on error for Addax benchmarks (but allow individual test failures)
set -e

# ============================================
# Addax 2-Round Interactive Throughput
# ============================================
echo ">>> Running Addax 2-round interactive throughput benchmarks..."
cd "$SCRIPT_DIR/addax/auction/throughput/build"

# Ensure binaries exist
if [ ! -f "./addax-server" ] || [ ! -f "./addax-client" ]; then
    echo "  Error: Addax binaries not found. Please build first:"
    echo "    cd ../ && cmake . -B build && cd build && make -j"
    exit 1
fi

# Generate interactive shares for 2-round protocol
# For 2-round: BUCKET-NUM = 100 (as per README)
# For 4-round: BUCKET-NUM = 10
# The script creates ${ROUND}-interactive-idx directory with 3*ROUND files
# Note: Throughput evaluation uses 96 bidders (100 is fine) and 1000 domain (100 buckets for 2-round)
BIDDER_NUM=96
BUCKET_NUM_2ROUND=100
ROUNDS_2ROUND=2

SHARES_DIR_INTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/shares"
COMMITS_DIR_INTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/commits"
IDX_DIR_INTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/${ROUNDS_2ROUND}-interactive-idx"

echo "  Generating interactive shares for 2-round protocol..."
echo "    Bidders: $BIDDER_NUM, Buckets: $BUCKET_NUM_2ROUND, Rounds: $ROUNDS_2ROUND"
cd "$SCRIPT_DIR/addax/auction/tools/build"
# Always regenerate to ensure clean state
sudo rm -rf "$SHARES_DIR_INTERACTIVE" "$COMMITS_DIR_INTERACTIVE" "$IDX_DIR_INTERACTIVE" 2>/dev/null
sudo bash adv-gen-sc-interactive.sh $BIDDER_NUM $BUCKET_NUM_2ROUND "$SHARES_DIR_INTERACTIVE" "$COMMITS_DIR_INTERACTIVE" $ROUNDS_2ROUND
echo "    Created directory: ${ROUNDS_2ROUND}-interactive-idx with 3*${ROUNDS_2ROUND} files"
cd "$SCRIPT_DIR/addax/auction/throughput/build"

# Load values to test (matching the expected results table)
# For 2-round: 5, 20, 25, 30, 50, 60, 65, 70, 80, 90, 100
# For non-interactive: 5, 10, 15, 18, 20, 30, 50
LOAD_VALUES_2ROUND=(5 20 25 30 50 60 65 70 80 90 100)
LOAD_VALUES_NONINTERACTIVE=(5 10 15 18 20 30 50)

RESULTS_DIR_2ROUND="$SCRIPT_DIR/addax/auction/throughput/results_2round"
mkdir -p "$RESULTS_DIR_2ROUND"

echo "  Testing Addax 2-round interactive with loads: ${LOAD_VALUES_2ROUND[*]}"
echo "  (Running sequentially, parallel=1 for server and client)"

for load in "${LOAD_VALUES_2ROUND[@]}"; do
    echo ""
    echo "  === Testing load: $load auctions ==="
    
    SERVER_DIR="$RESULTS_DIR_2ROUND/serv_op_1_${load}"
    CLIENT_DIR="$RESULTS_DIR_2ROUND/client_1_${load}"
    
    # Kill any existing servers/clients
    sudo pkill addax-server 2>/dev/null || true
    sudo pkill addax-client 2>/dev/null || true
    sleep 1
    
    # Start server (1 server, using interactive shares and ${ROUNDS_2ROUND}-interactive-idx, ${ROUNDS_2ROUND} rounds)
    echo "    Starting server..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-server.sh" 1 "$SERVER_DIR" "$SHARES_DIR_INTERACTIVE" "$IDX_DIR_INTERACTIVE" $ROUNDS_2ROUND &
    SERVER_PID=$!
    sleep 2
    
    # Start client (1 client, load auctions, ${ROUNDS_2ROUND} rounds)
    echo "    Starting client with load $load..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-client.sh" 1 $load "$CLIENT_DIR" $ROUNDS_2ROUND
    
    # Wait for completion (longer for higher loads)
    if [ $load -le 20 ]; then
        sleep 10
    elif [ $load -le 50 ]; then
        sleep 20
    else
        sleep 30
    fi
    
    # Kill server
    sudo pkill addax-server 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    # Calculate throughput
    if [ -d "$SERVER_DIR" ]; then
        echo "    Calculating throughput..."
        python3 ../cal-throughput.py "$SERVER_DIR" $load > "$RESULTS_DIR_2ROUND/throughput_1_${load}.txt" 2>&1 || true
    fi
    
    # Calculate latency
    if [ -d "$CLIENT_DIR" ]; then
        echo "    Calculating latency..."
        python3 ../cal-latency.py "$CLIENT_DIR" > "$RESULTS_DIR_2ROUND/latency_1_${load}.txt" 2>&1 || true
    fi
    
    sleep 1
done

echo ""
echo "  Addax 2-round interactive results saved to: $RESULTS_DIR_2ROUND"
echo ""

# ============================================
# Addax Non-Interactive Throughput
# ============================================
echo ">>> Running Addax non-interactive throughput benchmarks..."
cd "$SCRIPT_DIR/addax/auction/throughput/build"

# For non-interactive, use non-interactive shares
# Non-interactive uses 1000 buckets (for 1000 domain size)
BIDDER_NUM_NONINTERACTIVE=100
BUCKET_NUM_NONINTERACTIVE=1000

SHARES_DIR_NONINTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/shares_noninteractive"
COMMITS_DIR_NONINTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/commits_noninteractive"
IDX_BASE="$SCRIPT_DIR/addax/auction/tools/build"

echo "  Generating non-interactive shares (96 bidders, 1000 buckets)..."
cd "$SCRIPT_DIR/addax/auction/tools/build"
# Always regenerate to ensure clean state
sudo rm -rf "$SHARES_DIR_NONINTERACTIVE" "$COMMITS_DIR_NONINTERACTIVE" 2>/dev/null
mkdir -p "$SHARES_DIR_NONINTERACTIVE" "$COMMITS_DIR_NONINTERACTIVE"
bash adv-gen-sc.sh $BIDDER_NUM_NONINTERACTIVE $BUCKET_NUM_NONINTERACTIVE "$SHARES_DIR_NONINTERACTIVE" "$COMMITS_DIR_NONINTERACTIVE"
cd "$SCRIPT_DIR/addax/auction/throughput/build"

# For non-interactive, the idx files are flat (96-1000-s1-idx, etc.) in the base directory
# The server script expects a directory, so we point it to the base directory
IDX_DIR_NONINTERACTIVE="$IDX_BASE"
IDX_S1="$IDX_BASE/${BIDDER_NUM_NONINTERACTIVE}-${BUCKET_NUM_NONINTERACTIVE}-s1-idx"
IDX_S2="$IDX_BASE/${BIDDER_NUM_NONINTERACTIVE}-${BUCKET_NUM_NONINTERACTIVE}-s2-idx"
IDX_COMMIT="$IDX_BASE/${BIDDER_NUM_NONINTERACTIVE}-${BUCKET_NUM_NONINTERACTIVE}-commit-idx"

if [ ! -f "$IDX_S1" ] || [ ! -f "$IDX_S2" ] || [ ! -f "$IDX_COMMIT" ]; then
    echo "  Warning: Non-interactive idx files not found. Expected:"
    echo "    $IDX_S1"
    echo "    $IDX_S2"
    echo "    $IDX_COMMIT"
fi

RESULTS_DIR_NONINTERACTIVE="$SCRIPT_DIR/addax/auction/throughput/results_noninteractive"
mkdir -p "$RESULTS_DIR_NONINTERACTIVE"

echo "  Testing Addax non-interactive with loads: ${LOAD_VALUES_NONINTERACTIVE[*]}"
echo "  (Running sequentially, parallel=1 for server and client)"

for load in "${LOAD_VALUES_NONINTERACTIVE[@]}"; do
    echo ""
    echo "  === Testing load: $load auctions ==="
    
    SERVER_DIR="$RESULTS_DIR_NONINTERACTIVE/serv_op_1_${load}"
    CLIENT_DIR="$RESULTS_DIR_NONINTERACTIVE/client_1_${load}"
    
    # Kill any existing servers/clients
    sudo pkill addax-server 2>/dev/null || true
    sudo pkill addax-client 2>/dev/null || true
    sleep 1
    
    # Start server (1 server, using non-interactive shares, base idx dir)
    echo "    Starting server..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-server.sh" 1 "$SERVER_DIR" "$SHARES_DIR_NONINTERACTIVE" "$IDX_DIR_NONINTERACTIVE" 1 &
    SERVER_PID=$!
    sleep 2
    
    # Start client (1 client, load auctions)
    echo "    Starting client with load $load..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-client.sh" 1 $load "$CLIENT_DIR" 1
    
    # Wait for client processes to finish (they run in background)
    # Calculate wait time based on load: roughly 1 second per auction + buffer
    WAIT_TIME=$((load + 10))
    echo "    Waiting ${WAIT_TIME}s for client to complete..."
    sleep $WAIT_TIME
    
    # Wait for all client processes to finish
    while pgrep -f "addax-client" > /dev/null; do
        sleep 1
    done
    
    # Give a moment for logs to be written
    sleep 2
    
    # Kill server
    sudo pkill addax-server 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    # Calculate throughput
    if [ -d "$SERVER_DIR" ]; then
        echo "    Calculating throughput..."
        python3 ../cal-throughput.py "$SERVER_DIR" $load > "$RESULTS_DIR_NONINTERACTIVE/throughput_1_${load}.txt" 2>&1 || true
    fi
    
    # Calculate latency
    if [ -d "$CLIENT_DIR" ]; then
        echo "    Calculating latency..."
        python3 ../cal-latency.py "$CLIENT_DIR" > "$RESULTS_DIR_NONINTERACTIVE/latency_1_${load}.txt" 2>&1 || true
    fi
    
    sleep 1
done

echo ""
echo "  Addax non-interactive results saved to: $RESULTS_DIR_NONINTERACTIVE"
echo ""

echo "=========================================="
echo "Throughput benchmarks completed!"
echo "=========================================="
echo ""
echo "Results:"
echo "  - Obsidian: obsidian/throughput_results.csv"
echo "  - Addax 2-round interactive: addax/auction/throughput/results_2round/"
echo "  - Addax non-interactive: addax/auction/throughput/results_noninteractive/"
echo ""
echo "To generate plots, run:"
echo "  python3 plot_throughput.py"
