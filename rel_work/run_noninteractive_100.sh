#!/bin/bash

# Run 100 bidders non-interactive throughput test
# Usage: sudo ./run_noninteractive_100.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Running 100 Bidders Non-Interactive Throughput"
echo "=========================================="
echo ""

cd "$SCRIPT_DIR/addax/auction/throughput/build"

# Configuration
BIDDER_NUM=100
BUCKET_NUM=1000
LOAD_VALUES=(5 10 15 18 20 30 50)

SHARES_DIR="$SCRIPT_DIR/addax/auction/tools/build/shares_noninteractive"
COMMITS_DIR="$SCRIPT_DIR/addax/auction/tools/build/commits_noninteractive"
IDX_BASE="$SCRIPT_DIR/addax/auction/tools/build"
IDX_DIR="$IDX_BASE"

echo "Generating non-interactive shares (${BIDDER_NUM} bidders, ${BUCKET_NUM} buckets)..."
cd "$SCRIPT_DIR/addax/auction/tools/build"
sudo rm -rf "$SHARES_DIR" "$COMMITS_DIR" ${BIDDER_NUM}-${BUCKET_NUM}-*idx 2>/dev/null
mkdir -p "$SHARES_DIR" "$COMMITS_DIR"
bash adv-gen-sc.sh $BIDDER_NUM $BUCKET_NUM "$SHARES_DIR" "$COMMITS_DIR"
cd "$SCRIPT_DIR/addax/auction/throughput/build"

RESULTS_DIR="$SCRIPT_DIR/addax/auction/throughput/results_noninteractive_100"
mkdir -p "$RESULTS_DIR"

echo "Testing with loads: ${LOAD_VALUES[*]}"
echo ""

for load in "${LOAD_VALUES[@]}"; do
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
    
    # Calculate throughput
    if [ -d "$SERVER_DIR" ]; then
        echo "  Calculating throughput..."
        python3 ../cal-throughput.py "$SERVER_DIR" $load > "$RESULTS_DIR/throughput_1_${load}.txt" 2>&1 || true
    fi
    
    # Calculate latency
    if [ -d "$CLIENT_DIR" ]; then
        echo "  Calculating latency..."
        python3 ../cal-latency.py "$CLIENT_DIR" > "$RESULTS_DIR/latency_1_${load}.txt" 2>&1 || true
    fi
    
    echo "  Results saved to:"
    echo "    Throughput: $RESULTS_DIR/throughput_1_${load}.txt"
    echo "    Latency: $RESULTS_DIR/latency_1_${load}.txt"
    echo ""
done

echo "=========================================="
echo "100 bidders non-interactive test completed!"
echo "Results in: $RESULTS_DIR"
echo "=========================================="


