# Addax: A fast, private, and accountable ad exchange infrastructure

This repository contains the codes of the paper "Addax: A fast, private, and
accountable ad exchange infrastructure" in NSDI 2023.

## Setup

Please refer to [install.md](./install.md) for installation instructions.

## Code organization

+ Private and verifiable auction
    + [auction/addax-lib](./auction/addax-lib/):
      library codes of Addax.
    + [auction/tools](./auction/tools/):
    a tool used for evaluation.
    + [auction/micro](./auction/micro/):
    microbenchmark for bidder's costs.
    + [auction/auction-local-computation](./auction/auction-local-computation/):
      evaluation of local computation costs.
    + [auction/end-to-end-latency](./auction/end-to-end-latency/):
    evaluation of end-to-end latency.
    + [auction/throughput](./auction/throughput/):
    evaluation of throughput.
    + [auction/verify](./auction/verify/):
    evaluation of verification costs.

## Instructions for running

Please refer to the `README.md` files under each directory.

## Network Benchmarking with RTT Emulation

This section describes how to run Addax non-interactive auction benchmarks with network RTT emulation using `netem`.

### Prerequisites

1. **Build the auction-non-interactive binary:**
   ```bash
   cd auction/auction-local-computation
   cmake . -B build
   cd build
   make -j
   ```

2. **Generate share files for your configurations:**
   ```bash
   cd auction/tools
   cmake . -B build
   cd build
   make -j
   
   # Copy generators.txt to build directory
   cp ../../files/generators.txt .
   
   # Copy generation scripts
   cp ../adv-gen-sc.sh .
   cp ../adv-gen-sc-interactive.sh .
   
   # Make share-commit-gen executable
   chmod +x share-commit-gen
   
   # Generate shares for different configurations
   # Format: bash adv-gen-sc.sh <num_bidders> <bucket_num> <share_dir> <commit_dir>
   bash adv-gen-sc.sh 25 1000 shares_1000_25 commitments_1000_25
   bash adv-gen-sc.sh 50 1000 shares_1000_50 commitments_1000_50
   bash adv-gen-sc.sh 25 100 shares_100_25 commitments_100_25
   bash adv-gen-sc.sh 50 100 shares_100_50 commitments_100_50
   bash adv-gen-sc.sh 25 10000 shares_10000_25 commitments_10000_25
   bash adv-gen-sc.sh 50 10000 shares_10000_50 commitments_10000_50
   
   # For 100 bidders, use generic directory names (existing configs)
   bash adv-gen-sc.sh 100 1000 shares commitments  # Creates shares/ directory
   bash adv-gen-sc.sh 100 10000 shares_10000 commitments_10000
   bash adv-gen-sc.sh 100 100 shares_100 commitments_100
   ```

   This will create:
   - Share directories: `shares_<bucket>_<bidders>/` or `shares_<bucket>/`
   - Index files: `<bidders>-<bucket>-s1-idx` and `<bidders>-<bucket>-s2-idx`

### Running Network Benchmarks

1. **Single configuration:**
   ```bash
   sudo ./run_network_benchmark.sh <num_bidders> <bucket_num> <rtt_ms> <num_runs>
   ```
   
   Example:
   ```bash
   sudo ./run_network_benchmark.sh 100 1000 20 3
   ```
   
   This runs 3 iterations of the benchmark with:
   - 100 bidders
   - 1000 buckets
   - 20ms RTT (10ms one-way delay via netem)

2. **All configurations:**
   ```bash
   sudo ./run_all_benchmarks.sh [num_runs]
   ```
   
   This runs all configurations from the paper:
   - 100 bidders: 10000, 1000, 100 buckets
   - 50 bidders: 1000 buckets
   - 25 bidders: 1000 buckets
   - RTT values: 20ms, 40ms, 60ms
   - Default: 3 runs per configuration

### Output

Results are saved in CSV format in `auction/auction-local-computation/results/`:
- Filename: `addax_network_<bidders>_<buckets>_<rtt>ms.csv`
- Columns: `run,online_time_s,comm_bytes_mb,comm_bytes_kb`

Example output:
```
run,online_time_s,comm_bytes_mb,comm_bytes_kb
1,0.709308,0.20,206.21
2,0.698600,0.20,206.21
3,0.701234,0.20,206.21
```

### How It Works

1. **Netem Setup:** The script uses `tc qdisc` to add network delay to the loopback interface (`lo`)
   - RTT is split into one-way delay: `delay = RTT / 2`
   - Example: 20ms RTT → 10ms delay

2. **Process Execution:**
   - Publisher starts first and listens on ports 6666 and 6667
   - Server starts after a short delay and connects to publisher
   - Both processes communicate over TCP sockets with emulated RTT

3. **Result Parsing:**
   - Extracts `TIME: total:` for online time (seconds)
   - Extracts `GRAND TOTAL:` for communication (bytes)
   - Converts to MB and KB for convenience

### Troubleshooting

- **Permission denied:** Make sure `auction-non-interactive` is executable:
  ```bash
  chmod +x auction/auction-local-computation/build/auction-non-interactive
  ```

- **Share files not found:** Ensure you've generated shares for your configuration using `adv-gen-sc.sh`

- **Port already in use:** The script uses ports 6666 and 6667. Make sure no other processes are using them.

- **Netem cleanup:** If the script is interrupted, manually clean up netem:
  ```bash
  sudo tc qdisc del dev lo root
  ```
