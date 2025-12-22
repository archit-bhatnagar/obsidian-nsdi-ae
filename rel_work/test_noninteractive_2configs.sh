#!/bin/bash

# Test non-interactive with 2 configs to check logs and latency calculation
# Usage: sudo ./test_noninteractive_2configs.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Testing Non-Interactive (2 Configs)"
echo "=========================================="
echo ""

cd "$SCRIPT_DIR/addax/auction/throughput/build"

# Configuration
BIDDER_NUM=100
BUCKET_NUM=1000
TEST_LOADS=(5 20)  # Just 2 loads for testing

SHARES_DIR="$SCRIPT_DIR/addax/auction/tools/build/shares_noninteractive"
COMMITS_DIR="$SCRIPT_DIR/addax/auction/tools/build/commits_noninteractive"
IDX_BASE="$SCRIPT_DIR/addax/auction/tools/build"
IDX_DIR="$IDX_BASE"

echo "Generating non-interactive shares (${BIDDER_NUM} bidders, ${BUCKET_NUM} buckets)..."
cd "$SCRIPT_DIR/addax/auction/tools/build"
if [ ! -d "$SHARES_DIR" ] || [ ! -f "${IDX_BASE}/${BIDDER_NUM}-${BUCKET_NUM}-s1-idx" ]; then
    sudo rm -rf "$SHARES_DIR" "$COMMITS_DIR" ${BIDDER_NUM}-${BUCKET_NUM}-*idx 2>/dev/null
    mkdir -p "$SHARES_DIR" "$COMMITS_DIR"
    bash adv-gen-sc.sh $BIDDER_NUM $BUCKET_NUM "$SHARES_DIR" "$COMMITS_DIR"
fi
cd "$SCRIPT_DIR/addax/auction/throughput/build"

RESULTS_DIR="$SCRIPT_DIR/addax/auction/throughput/results_noninteractive_test"
mkdir -p "$RESULTS_DIR"

echo "Testing with loads: ${TEST_LOADS[*]}"
echo ""

for load in "${TEST_LOADS[@]}"; do
    echo "=== Testing load: $load auctions ==="
    
    SERVER_DIR="$RESULTS_DIR/serv_op_1_${load}"
    CLIENT_DIR="$RESULTS_DIR/client_1_${load}"
    
    sudo pkill addax-server 2>/dev/null || true
    sudo pkill addax-client 2>/dev/null || true
    sleep 1
    
    echo "  Starting server..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-server.sh" 1 "$SERVER_DIR" "$SHARES_DIR" "$IDX_DIR" 1 &
    SERVER_PID=$!
    sleep 3
    
    echo "  Starting client with load $load..."
    bash "$SCRIPT_DIR/addax/auction/throughput/build/run-addax-client.sh" 1 $load "$CLIENT_DIR" 1
    
    # Wait for client to finish
    WAIT_TIME=$((load + 10))
    echo "  Waiting ${WAIT_TIME}s for client to complete..."
    sleep $WAIT_TIME
    while pgrep -f "addax-client" > /dev/null; do
        sleep 1
    done
    sleep 2
    
    sudo pkill addax-server 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    echo ""
    echo "  === Log Analysis for load $load ==="
    
    # Check client logs
    echo "  Client log files:"
    ls -lh "$CLIENT_DIR"/*.txt 2>/dev/null | head -5 || echo "    No client log files found"
    
    echo ""
    echo "  Sample client log content (first 20 lines):"
    if [ -f "$CLIENT_DIR/6667.txt" ]; then
        head -20 "$CLIENT_DIR/6667.txt" | sed 's/^/    /'
    else
        echo "    Client log file not found"
    fi
    
    echo ""
    echo "  Calculating latency..."
    if [ -d "$CLIENT_DIR" ]; then
        python3 ../cal-latency.py "$CLIENT_DIR" > "$RESULTS_DIR/latency_1_${load}.txt" 2>&1 || true
        echo "  Latency results:"
        cat "$RESULTS_DIR/latency_1_${load}.txt" | sed 's/^/    /'
    fi
    
    echo ""
    echo "  Server log files:"
    ls -lh "$SERVER_DIR"/*.txt 2>/dev/null | head -5 || echo "    No server log files found"
    
    echo ""
    echo "  Sample server log content (last 20 lines):"
    if [ -f "$SERVER_DIR/6667.txt" ]; then
        tail -20 "$SERVER_DIR/6667.txt" | sed 's/^/    /'
    else
        echo "    Server log file not found"
    fi
    
    echo ""
    echo "  Calculating throughput..."
    if [ -d "$SERVER_DIR" ]; then
        python3 ../cal-throughput.py "$SERVER_DIR" $load > "$RESULTS_DIR/throughput_1_${load}.txt" 2>&1 || true
        echo "  Throughput results:"
        cat "$RESULTS_DIR/throughput_1_${load}.txt" | sed 's/^/    /'
    fi
    
    echo ""
    echo "  ---"
    echo ""
done

echo "=========================================="
echo "Test completed!"
echo "Results in: $RESULTS_DIR"
echo "=========================================="


