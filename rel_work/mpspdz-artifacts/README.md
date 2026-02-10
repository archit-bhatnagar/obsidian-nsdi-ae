# MP-SPDZ Artifacts for Obsidian Evaluation

This directory contains the custom MPC programs and data files for evaluating the Vickrey auction implementation.

## Directory Structure

- `Programs/` - Custom MPC source files (`.mpc` programs)
- `Player-Data/` - Input data files for the MPC programs

## Setup Instructions

### 1. Install MP-SPDZ

**Option A - Clone from GitHub (recommended):**

```bash
cd rel_work/
git clone https://github.com/data61/MP-SPDZ.git mp-spdz-0.3.9
cd mp-spdz-0.3.9
git checkout v0.3.9
make -j8 tldr
```

**Option B - Download pre-packaged release (if GitHub build fails):**

```bash
cd rel_work/
wget https://github.com/data61/MP-SPDZ/releases/download/v0.3.9/mp-spdz-0.3.9.tar.xz
tar -xf mp-spdz-0.3.9.tar.xz
cd mp-spdz-0.3.9
Scripts/tldr.sh
```

For detailed installation instructions, see the [MP-SPDZ documentation](https://github.com/data61/MP-SPDZ).

### 2. Copy Artifacts

Use the provided setup script to copy the artifacts into your MP-SPDZ installation:

```bash
cd rel_work/mpspdz-artifacts/
./setup_mpspdz.sh
```

Or manually copy the files:

```bash
# From the mpspdz-artifacts directory
cp Programs/* ../mp-spdz-0.3.9/Programs/Source/
cp Player-Data/* ../mp-spdz-0.3.9/Player-Data/
cp Persistence/* ../mp-spdz-0.3.9/Persistence/
```

### 3. Compile and Run

Compile the Vickrey auction program:

```bash
cd ../mp-spdz-0.3.9/
./compile.py vickrey
```

Run with the MASCOT protocol:

```bash
Scripts/mascot.sh -v vickrey
```

To clear secret share files between runs:

```bash
./clear_persist.sh
```

## Files Included

### Programs
- `vickrey.mpc` - Vickrey auction MPC program

### Player-Data
- `bids_party0.txt` - Bid data for party 0
- `bids_party1.txt` - Bid data for party 1

### Persistence
- `Transactions-P0.data` - Persistence data for party 0
- `Transactions-P1.data` - Persistence data for party 1

### Scripts
- `clear_persist.sh` - Script to clear secret share files between runs

## Network Benchmarking with RTT Emulation

This section describes how to run MP-SPDZ Vickrey auction benchmarks with network RTT emulation using `netem`.

### Prerequisites

1. **Install and build MP-SPDZ:**
   ```bash
   cd rel_work/
   git clone https://github.com/data61/MP-SPDZ.git mp-spdz-0.3.9
   cd mp-spdz-0.3.9
   git checkout v0.3.9
   make -j8 tldr
   ```

2. **Copy artifacts:**
   ```bash
   cd ../mpspdz-artifacts/
   ./setup_mpspdz.sh
   ```

3. **Compile the Vickrey program:**
   ```bash
   cd ../mp-spdz-0.3.9/
   ./compile.py vickrey
   ```

### Running Network Benchmarks

1. **Copy the benchmark scripts:**
   ```bash
   cp ../mpspdz-artifacts/run_network_benchmark.sh .
   cp ../mpspdz-artifacts/run_all_benchmarks.sh .
   chmod +x run_network_benchmark.sh run_all_benchmarks.sh
   ```

2. **Run a single configuration:**
   ```bash
   sudo ./run_network_benchmark.sh <num_bidders> <rtt_ms> <num_runs>
   ```
   
   Example :
   ```bash
   sudo ./run_network_benchmark.sh 100 100 20 1
   ```
   
   This runs 3 iterations with:
   - 100 bidders
   - 100 bid domain
   - 20ms RTT (10ms one-way delay via netem)

(Note: The original vickrey optimization needs a number of inputs divisible bu num_threads so the script actually does the closest appproximation of num_bidders for that)
### Output

Results are saved in CSV format in `results/`:
- Filename: `mpspdz_network_<bidders>_<rtt>ms.csv`
- Columns: `run,online_time_s,comm_bytes_mb,comm_bytes_kb`

### How It Works

1. **Netem Setup:** Uses `tc qdisc` to add network delay to loopback interface
   - RTT is split into one-way delay: `delay = RTT / 2`
   - Example: 20ms RTT → 10ms delay

2. **Process Execution:**
   - Runs MP-SPDZ with MASCOT protocol
   - Two parties communicate over TCP with emulated RTT
   - Logs are saved to `logs/` directory

3. **Result Parsing:**
   - Extracts timing from log files
   - Extracts communication statistics
   - Converts to MB and KB for convenience

### Troubleshooting

- **Permission denied:** Make sure script is executable:
  ```bash
  chmod +x run_network_benchmark.sh
  ```

- **MP-SPDZ build fails (boost download errors):** If cloning from GitHub and running `make -j8 tldr` fails with boost-related errors, use the pre-packaged source release instead:
  ```bash
  cd rel_work/
  wget https://github.com/data61/MP-SPDZ/releases/download/v0.3.9/mp-spdz-0.3.9.tar.xz
  tar -xf mp-spdz-0.3.9.tar.xz
  cd mp-spdz-0.3.9
  Scripts/tldr.sh
  ```
  Then continue with step 2 (Copy Artifacts) above.

- **Program not compiled:** Compile the Vickrey program:
  ```bash
  ./compile.py vickrey
  ```

- **Netem cleanup:** If interrupted, manually clean up:
  ```bash
  sudo tc qdisc del dev lo root
  ```

- **Parsing errors:** Check log files in `logs/` directory for actual output format

## Notes

- The persistence files are optional and only needed for certain protocols
- Evaluators should install MP-SPDZ themselves to ensure a clean environment
- All source files in this directory are the custom artifacts for the evaluation
- The benchmark script should be copied to the MP-SPDZ installation directory before running

