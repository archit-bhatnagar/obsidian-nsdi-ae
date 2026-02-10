#!/bin/bash
# Run microbenchmarks for Obsidian offline and online phases

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BINARY="./target/release/dpf_benchmark"
RESULTS_DIR="results"
RESULTS_FILE="${RESULTS_DIR}/microbenchmark_results.csv"
RAW_OUTPUT="${RESULTS_DIR}/microbenchmark_raw.txt"
NUM_RUNS=${1:-5}  # Default 5 runs, can be overridden
SLEEP_BETWEEN_RUNS=0.5

mkdir -p "$RESULTS_DIR"

# Build if needed
if [ ! -f "$BINARY" ]; then
    echo "Building dpf_benchmark..."
    export RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native"
    cargo build --release --bin dpf_benchmark
fi

export RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native"

# Initialize CSV
echo "num_bidders,domain_size,run,phase,time_ns,time_ms,time_us,comm_bytes" > "$RESULTS_FILE"
> "$RAW_OUTPUT"

# Test configurations
declare -a variant1_bidders=(100 100 100 100 100 100)
declare -a variant1_domains=(128 256 512 1024 2048 4096)

declare -a variant2_bidders=(25 50 100 200 400 800)
declare -a variant2_domains=(1024 1024 1024 1024 1024 1024)

run_benchmark() {
    local num_bidders=$1
    local domain_size=$2
    
    echo "Running: $num_bidders bidders, domain $domain_size ($NUM_RUNS runs)"
    
    TEMP_OUTPUT=$(mktemp)
    "$BINARY" "$num_bidders" "$domain_size" "$NUM_RUNS" 2>&1 | tee "$TEMP_OUTPUT" | tee -a "$RAW_OUTPUT"
    
    # Parse results
    python3 << PYTHON_SCRIPT
import re
import sys

def parse_duration(duration_str):
    match = re.match(r'([\d.]+)(\w+)', duration_str.strip())
    if not match:
        return None
    value = float(match.group(1))
    unit = match.group(2).lower()
    if unit == 'ns':
        return value
    elif unit in ['us', 'µs']:
        return value * 1000
    elif unit == 'ms':
        return value * 1_000_000
    elif unit == 's':
        return value * 1_000_000_000
    return None

num_bidders = $num_bidders
domain_size = $domain_size
results_file = "$RESULTS_FILE"
temp_output = "$TEMP_OUTPUT"

runs = []
current_run = None

with open(temp_output, 'r') as f:
    lines = f.readlines()

for line in lines:
    if '=== Run' in line:
        match = re.search(r'Run (\d+)', line)
        if match:
            if current_run is not None:
                runs.append(current_run)
            current_run = {'run': int(match.group(1)), 'preprocess': None, 'online': None, 'preprocess_comm': None, 'online_comm': None}
    
    if 'Pre-processing took:' in line and current_run is not None:
        match = re.search(r'Pre-processing took: ([\d.]+)(\w+)', line)
        if match:
            duration_str = match.group(1) + match.group(2)
            time_ns = parse_duration(duration_str)
            if time_ns is not None:
                current_run['preprocess'] = time_ns
    
    if 'Online time:' in line and current_run is not None:
        match = re.search(r'Online time: ([\d.]+)(\w+)', line)
        if match:
            duration_str = match.group(1) + match.group(2)
            time_ns = parse_duration(duration_str)
            if time_ns is not None:
                current_run['online'] = time_ns
    
    if 'Preprocessing communication:' in line and current_run is not None:
        match = re.search(r'Preprocessing communication: (\d+) bytes', line)
        if match:
            current_run['preprocess_comm'] = int(match.group(1))
    
    if 'Online communication:' in line and current_run is not None:
        match = re.search(r'Online communication: (\d+) bytes', line)
        if match:
            current_run['online_comm'] = int(match.group(1))

if current_run is not None:
    runs.append(current_run)

with open(results_file, 'a') as f:
    for run in runs:
        if run['preprocess'] is not None:
            comm_bytes = run['preprocess_comm'] if run['preprocess_comm'] is not None else 0
            f.write(f"{num_bidders},{domain_size},{run['run']},preprocess,{run['preprocess']:.0f},{run['preprocess']/1_000_000:.3f},{run['preprocess']/1_000:.3f},{comm_bytes}\n")
        if run['online'] is not None:
            comm_bytes = run['online_comm'] if run['online_comm'] is not None else 0
            f.write(f"{num_bidders},{domain_size},{run['run']},online,{run['online']:.0f},{run['online']/1_000_000:.3f},{run['online']/1_000:.3f},{comm_bytes}\n")

print(f"✓ Completed {len(runs)} runs")
PYTHON_SCRIPT

    rm -f "$TEMP_OUTPUT"
    
    # Sleep between runs
    sleep "$SLEEP_BETWEEN_RUNS"
}

# Run variant 1: varying domain
echo "Running variant 1: fixed 100 bidders, varying domain..."
for i in "${!variant1_bidders[@]}"; do
    run_benchmark "${variant1_bidders[$i]}" "${variant1_domains[$i]}"
done

# Run variant 2: varying bidders
echo ""
echo "Running variant 2: fixed 1024 domain, varying bidders..."
for i in "${!variant2_bidders[@]}"; do
    run_benchmark "${variant2_bidders[$i]}" "${variant2_domains[$i]}"
done

echo ""
echo "All benchmarks completed!"
echo "Results saved to: $RESULTS_FILE"
