#!/bin/bash

# Test non-interactive with higher loads to see log patterns
# Usage: sudo ./test_noninteractive_higher_loads.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Testing Non-Interactive (Higher Loads)"
echo "=========================================="
echo ""

cd "$SCRIPT_DIR/addax/auction/throughput/build"

# Configuration
BIDDER_NUM=100
BUCKET_NUM=1000
TEST_LOADS=(5 10 15 18 20 30 50)  # Match expected results

SHARES_DIR="$SCRIPT_DIR/addax/auction/tools/build/shares_noninteractive"
COMMITS_DIR="$SCRIPT_DIR/addax/auction/tools/build/commits_noninteractive"
IDX_BASE="$SCRIPT_DIR/addax/auction/tools/build"
IDX_DIR="$IDX_BASE"

echo "Using existing shares (${BIDDER_NUM} bidders, ${BUCKET_NUM} buckets)..."
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
    
    # Wait for client to finish - longer for higher loads
    WAIT_TIME=$((load * 2 + 20))
    echo "  Waiting ${WAIT_TIME}s for client to complete..."
    sleep $WAIT_TIME
    while pgrep -f "addax-client" > /dev/null; do
        sleep 1
    done
    sleep 3
    
    sudo pkill addax-server 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    echo ""
    echo "  === Full Client Log for load $load ==="
    if [ -f "$CLIENT_DIR/6667.txt" ]; then
        cat "$CLIENT_DIR/6667.txt"
    else
        echo "    Client log file not found"
    fi
    
    echo ""
    echo "  === Full Server Log (last 50 lines) for load $load ==="
    if [ -f "$SERVER_DIR/6667.txt" ]; then
        tail -50 "$SERVER_DIR/6667.txt"
    else
        echo "    Server log file not found"
    fi
    
    echo ""
    echo "  ---"
    echo ""
done

echo "=========================================="
echo "Test completed!"
echo "=========================================="

