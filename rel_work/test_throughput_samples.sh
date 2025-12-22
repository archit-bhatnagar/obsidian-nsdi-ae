#!/bin/bash

# Quick test script to run a few samples for sanity check
# Usage: sudo ./test_throughput_samples.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Testing Throughput Samples (Sanity Check)"
echo "=========================================="
echo ""

# Test loads (small subset)
TEST_LOADS_2ROUND=(5 20 50)
TEST_LOADS_NONINTERACTIVE=(5 10 20)

# Use 100 bidders for non-interactive
BIDDER_NUM_NONINTERACTIVE=100
BUCKET_NUM_NONINTERACTIVE=1000

cd "$SCRIPT_DIR/addax/auction/throughput/build"

# Ensure binaries exist
if [ ! -f "./addax-server" ] || [ ! -f "./addax-client" ]; then
    echo "Error: Addax binaries not found. Please build first."
    exit 1
fi

# ============================================
# Test Addax 2-Round Interactive
# ============================================
echo ">>> Testing Addax 2-round interactive (samples)..."
echo ""

SHARES_DIR_INTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/shares"
IDX_DIR_INTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/2-interactive-idx"

# Check if shares exist
if [ ! -d "$SHARES_DIR_INTERACTIVE" ] || [ ! -d "$IDX_DIR_INTERACTIVE" ]; then
    echo "  Generating 2-round shares (96 bidders, 100 buckets)..."
    cd "$SCRIPT_DIR/addax/auction/tools/build"
    sudo rm -rf shares commits 2-interactive-idx 2>/dev/null
    sudo bash adv-gen-sc-interactive.sh 96 100 ./shares ./commits 2
    cd "$SCRIPT_DIR/addax/auction/throughput/build"
fi

RESULTS_DIR_2ROUND="$SCRIPT_DIR/addax/auction/throughput/results_2round_test"
mkdir -p "$RESULTS_DIR_2ROUND"

for load in "${TEST_LOADS_2ROUND[@]}"; do
    echo "  === Testing load: $load auctions ==="
    
    SERVER_DIR="$RESULTS_DIR_2ROUND/serv_op_1_${load}"
    CLIENT_DIR="$RESULTS_DIR_2ROUND/client_1_${load}"
    
    sudo pkill addax-server 2>/dev/null || true
    sudo pkill addax-client 2>/dev/null || true
    sleep 1
    
    echo "    Starting server..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-server.sh" 1 "$SERVER_DIR" "$SHARES_DIR_INTERACTIVE" "$IDX_DIR_INTERACTIVE" 2 > /dev/null 2>&1 &
    SERVER_PID=$!
    sleep 3
    
    echo "    Starting client..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-client.sh" 1 $load "$CLIENT_DIR" 2
    
    # Wait for client to finish
    WAIT_TIME=$((load + 10))
    sleep $WAIT_TIME
    while pgrep -f "addax-client" > /dev/null; do
        sleep 1
    done
    sleep 2
    
    sudo pkill addax-server 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    echo "    Results:"
    if [ -d "$SERVER_DIR" ]; then
        THROUGHPUT=$(python3 ../cal-throughput.py "$SERVER_DIR" $load 2>&1 | grep "total throughput" | awk '{print $NF}')
        echo "      Throughput: $THROUGHPUT auctions/sec"
    fi
    
    if [ -d "$CLIENT_DIR" ]; then
        LATENCY=$(python3 ../cal-latency.py "$CLIENT_DIR" 2>&1 | grep -E "median|99%")
        echo "      $LATENCY"
    fi
    echo ""
done

echo ""

# ============================================
# Test Addax Non-Interactive
# ============================================
echo ">>> Testing Addax non-interactive (samples)..."
echo ""

SHARES_DIR_NONINTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/shares_noninteractive"
COMMITS_DIR_NONINTERACTIVE="$SCRIPT_DIR/addax/auction/tools/build/commits_noninteractive"
IDX_BASE="$SCRIPT_DIR/addax/auction/tools/build"
IDX_DIR_NONINTERACTIVE="$IDX_BASE"

# Generate non-interactive shares if needed
if [ ! -d "$SHARES_DIR_NONINTERACTIVE" ] || [ ! -f "$IDX_BASE/${BIDDER_NUM_NONINTERACTIVE}-${BUCKET_NUM_NONINTERACTIVE}-s1-idx" ]; then
    echo "  Generating non-interactive shares (${BIDDER_NUM_NONINTERACTIVE} bidders, ${BUCKET_NUM_NONINTERACTIVE} buckets)..."
    cd "$SCRIPT_DIR/addax/auction/tools/build"
    sudo rm -rf shares_noninteractive commits_noninteractive ${BIDDER_NUM_NONINTERACTIVE}-${BUCKET_NUM_NONINTERACTIVE}-*idx 2>/dev/null
    mkdir -p shares_noninteractive commits_noninteractive
    bash adv-gen-sc.sh $BIDDER_NUM_NONINTERACTIVE $BUCKET_NUM_NONINTERACTIVE shares_noninteractive commits_noninteractive
    cd "$SCRIPT_DIR/addax/auction/throughput/build"
fi

IDX_DIR_NONINTERACTIVE="$IDX_BASE"

RESULTS_DIR_NONINTERACTIVE="$SCRIPT_DIR/addax/auction/throughput/results_noninteractive_test"
mkdir -p "$RESULTS_DIR_NONINTERACTIVE"

for load in "${TEST_LOADS_NONINTERACTIVE[@]}"; do
    echo "  === Testing load: $load auctions ==="
    
    SERVER_DIR="$RESULTS_DIR_NONINTERACTIVE/serv_op_1_${load}"
    CLIENT_DIR="$RESULTS_DIR_NONINTERACTIVE/client_1_${load}"
    
    sudo pkill addax-server 2>/dev/null || true
    sudo pkill addax-client 2>/dev/null || true
    sleep 1
    
    echo "    Starting server..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-server.sh" 1 "$SERVER_DIR" "$SHARES_DIR_NONINTERACTIVE" "$IDX_DIR_NONINTERACTIVE" 1 > /dev/null 2>&1 &
    SERVER_PID=$!
    sleep 3
    
    echo "    Starting client..."
    # Non-interactive uses r=1
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-client.sh" 1 $load "$CLIENT_DIR" 1
    
    # Wait for client to finish
    WAIT_TIME=$((load + 10))
    sleep $WAIT_TIME
    while pgrep -f "addax-client" > /dev/null; do
        sleep 1
    done
    sleep 2
    
    sudo pkill addax-server 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    echo "    Results:"
    if [ -d "$SERVER_DIR" ]; then
        THROUGHPUT=$(python3 ../cal-throughput.py "$SERVER_DIR" $load 2>&1 | grep "total throughput" | awk '{print $NF}')
        echo "      Throughput: $THROUGHPUT auctions/sec"
    fi
    
    if [ -d "$CLIENT_DIR" ]; then
        LATENCY=$(python3 ../cal-latency.py "$CLIENT_DIR" 2>&1 | grep -E "median|99%")
        echo "      $LATENCY"
    fi
    echo ""
done

echo "=========================================="
echo "Sample tests completed!"
echo "=========================================="
echo ""
echo "If these numbers look reasonable, we can run the full benchmark."

