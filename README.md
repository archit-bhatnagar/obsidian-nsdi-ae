# Artifact Evaluation: Secure Vickrey Auctions for Online Advertising

This artifact contains the implementation and evaluation code for **Obsidian**, a system for secure Vickrey auctions with bid confidentiality and bidder anonymity. This README provides detailed instructions for reproducing all experimental results from the paper.

## Table of Contents

1. [Overview](#overview)
2. [Directory Structure](#directory-structure)
3. [System Requirements](#system-requirements)
4. [Setup Instructions](#setup-instructions)
5. [Reproducing Experiments](#reproducing-experiments)
   - [Figure 4: Microbenchmarks](#figure-4-microbenchmarks)
   - [Figures 5, 7, 8, 9: Network Benchmarks (Latency & Communication)](#figures-5-7-8-9-network-benchmarks-latency--communication)
   - [Figure 10: Throughput Comparison](#figure-10-throughput-comparison)
6. [Expected Outputs](#expected-outputs)
7. [Artifact Description](#artifact-description)

## Overview

This artifact enables reproduction of all experimental results from the paper, including:

- **Microbenchmarks** (Figure 4): Performance analysis of Obsidian's offline and online phases
- **Network Benchmarks** (Figures 5, 7, 8, 9): Comparison of latency and communication across Obsidian, Addax, and MP-SPDZ
- **Throughput Evaluation** (Figure 10): Throughput and latency analysis for Obsidian and Addax protocols

**Note on SEAL**: SEAL is excluded from the network benchmark plots (Figures 5, 7, 8, 9) because it takes significantly longer to run (often exceeding hours for larger configurations) and performs much worse than the other systems. However, the SEAL benchmark code is available in `rel_work/seal-auction/` for evaluators who wish to run it separately.

## Directory Structure

```
obsidian-nsdi/
├── README.md                          # This file
├── nsdi26fall-paper273 (1).pdf       # The paper
│
├── obsidian/                          # Obsidian implementation
│   ├── src/                           # Rust source code
│   ├── results/                       # Benchmark results (CSV files)
│   ├── run_microbenchmarks.sh         # Run microbenchmarks (Fig 4)
│   ├── run_network_benchmark.sh       # Run network benchmarks
│   ├── bench_tput.sh                  # Run throughput benchmarks
│   ├── plot_microbenchmarks.py        # Generate Fig 4 plots
│   └── README.md                      # Obsidian-specific README
│
└── rel_work/                          # Related work comparison
    ├── README.md                      # Network benchmarking guide
    ├── addax/                         # Addax implementation
    │   └── auction/
    │       ├── auction-local-computation/  # Network benchmarks
    │       └── throughput/                 # Throughput benchmarks
    ├── mp-spdz-0.3.9/                # MP-SPDZ implementation
    ├── seal-auction/                  # SEAL implementation (excluded from plots)
    ├── run_all_benchmarks.sh          # Run all network benchmarks
    ├── run_required_benchmarks.sh     # Run only required configs
    ├── run_throughput_benchmarks.sh   # Run throughput benchmarks
    ├── plot_network_benchmarks.py     # Generate Figs 5, 7, 8, 9
    ├── plot_throughput.py             # Generate Fig 10
    └── plots/                         # Generated plots
        ├── comm_vs_bidders.png       # Fig 7
        ├── comm_vs_domain.png         # Fig 8
        ├── time_vs_bidders.png        # Fig 5
        ├── time_vs_domain.png         # Fig 9
        └── latency_vs_throughput.png  # Fig 10
```

## System Requirements

### Minimal Setup

- **Operating System**: Linux (tested on Ubuntu 20.04+)
- **CPU**: x86_64 architecture
- **Memory**: 8GB RAM minimum (16GB recommended)
- **Disk Space**: ~5GB for all dependencies and results
- **Network**: Internet connection for initial setup (downloading dependencies)

### Software Dependencies

- **Rust** (for Obsidian): Install via [rustup](https://rustup.rs/)
- **Python 3** (for plotting): `python3` with `matplotlib`
- **Build tools**: `gcc`, `g++`, `make`, `cmake`
- **Network tools**: `tc` (traffic control) for RTT emulation (requires `sudo`)
- **Additional**: `bc` (calculator), `taskset` (CPU affinity)

### Root/Sudo Access

Network benchmarks require `sudo` access to:
- Emulate network latency using `tc` (traffic control)
- Bind to network ports
- Set CPU affinity

## Setup Instructions

### 1. Clone and Navigate

```bash
cd obsidian-nsdi
```

### 2. Build Obsidian

```bash
cd obsidian
export RUSTFLAGS+="-C target-cpu=native"
cargo build --release
```

This builds all Obsidian binaries including:
- `dpf_benchmark` (microbenchmarks)
- `dpf_run_comm` (network benchmarks)
- `dpf_tput` (throughput benchmarks)

### 3. Setup Related Work Systems

The related work systems (Addax, MP-SPDZ) are already included in `rel_work/`. Each system has its own setup:

**Addax**: Pre-built binaries should be available. If not, see `rel_work/addax/README.md`.

**MP-SPDZ**: Pre-configured. If needed, see `rel_work/mpspdz-artifacts/README.md`.

**SEAL**: Available in `rel_work/seal-auction/` but not required for main plots.

### 4. Install Python Dependencies

```bash
pip3 install --user matplotlib numpy
```

## Reproducing Experiments

### Figure 4: Microbenchmarks

**Description**: Evaluates Obsidian's offline (preprocessing) and online phases across varying numbers of bidders and domain sizes.

**Steps**:

1. **Run microbenchmarks**:
   ```bash
   cd obsidian
   ./run_microbenchmarks.sh [num_runs]
   ```
   - Default: 5 runs per configuration
   - Tests two variants:
     - **Variant 1**: Fixed 100 bidders, varying domain (128, 256, 512, 1024, 2048, 4096)
     - **Variant 2**: Fixed 1024 domain, varying bidders (25, 50, 100, 200, 400, 800)
   - Results saved to: `obsidian/results/microbenchmark_results.csv`

2. **Generate plots**:
   ```bash
   python3 plot_microbenchmarks.py
   ```
   - Generates 4 PDF files:
     - `scale_domain.pdf` - Timing vs domain (100 bidders)
     - `scale_bidders.pdf` - Timing vs bidders (1024 domain)
     - `communication_domain.pdf` - Communication vs domain (100 bidders)
     - `communication_bidders.pdf` - Communication vs bidders (1024 domain)

**Expected Runtime**: ~10-15 minutes for all configurations

**Output**: `obsidian/results/microbenchmark_results.csv` and 4 PDF plots

---

### Figures 5, 7, 8, 9: Network Benchmarks (Latency & Communication)

**Description**: Compares Obsidian, Addax, and MP-SPDZ on latency and communication under different network conditions (20ms and 40ms RTT).

**Note**: SEAL is excluded from these plots due to extremely long runtimes and poor performance. The SEAL code is available in `rel_work/seal-auction/` for separate evaluation.

**Configurations**:
- **Varying bidders** (domain fixed at 1000): 25, 50, 100 bidders, RTT: 20ms, 40ms
- **Varying domain** (bidders fixed at 100): 100, 1000, 10000 domains, RTT: 20ms, 40ms

**Steps**:

1. **Run all required benchmarks** (recommended for first run):
   ```bash
   cd rel_work
   sudo ./run_required_benchmarks.sh [num_runs]
   ```
   - Default: 3 runs per configuration
   - Runs only the 10 unique configurations needed for the 4 plots
   - Results saved to system-specific directories:
     - Obsidian: `obsidian/results/network_benchmark_*.csv`
     - Addax: `addax/auction/auction-local-computation/results/addax_network_*.csv`
     - MP-SPDZ: `mp-spdz-0.3.9/results/mpspdz_network_*.csv`

2. **Alternative: Run all benchmarks** (if you want complete data):
   ```bash
   sudo ./run_all_benchmarks.sh [num_runs]
   ```

3. **Generate plots**:
   ```bash
   python3 plot_network_benchmarks.py
   ```
   - Generates 4 PNG files in `rel_work/plots/`:
     - `comm_vs_bidders.png` - Communication vs Number of Bidders (Fig 7)
     - `comm_vs_domain.png` - Communication vs Domain Size (Fig 8)
     - `time_vs_bidders.png` - Time vs Number of Bidders with 20ms/40ms RTT (Fig 5)
     - `time_vs_domain.png` - Time vs Domain Size with 20ms/40ms RTT (Fig 9)

**Expected Runtime**: 
- Required benchmarks: ~30-45 minutes
- All benchmarks: ~1-2 hours

**Output**: CSV files in system-specific result directories and 4 PNG plots

---

### Figure 10: Throughput Comparison

**Description**: Evaluates throughput and latency for Obsidian and Addax (2-round interactive and non-interactive protocols) under varying load.

**Steps**:

1. **Run throughput benchmarks**:
   ```bash
   cd rel_work
   sudo ./run_throughput_benchmarks.sh
   ```
   - Runs three sets of benchmarks:
     - **Obsidian**: Single-core throughput with varying load (1-500 auctions/sec)
     - **Addax 2-Round Interactive**: 96 bidders, 100 buckets, loads: 5, 20, 25, 30, 50, 60, 65, 70, 80, 90, 100
     - **Addax Non-Interactive**: 96 bidders, 1000 buckets, loads: 5, 10, 15, 18, 20, 30, 50
   - **Note**: Addax non-interactive uses 96 bidders (instead of 100) due to a bug with non-quadruple numbers and parsing in Addax
   - Results saved to:
     - Obsidian: `obsidian/throughput_results.csv`
     - Addax 2-Round: `addax/auction/throughput/results_2round/`
     - Addax Non-Interactive: `addax/auction/throughput/results_noninteractive/`

2. **Generate plot**:
   ```bash
   python3 plot_throughput.py
   ```
   - Generates `latency_vs_throughput.png` in `rel_work/plots/`
   - Shows p50 and p99 latency vs throughput for all three systems

**Expected Runtime**: ~1-2 hours (depends on system performance)

**Output**: CSV files and `latency_vs_throughput.png`

**Expected Values**:

The script should produce results similar to:

| Addax Non-Interactive | Addax 2-Round Interactive |
|----------------------|---------------------------|
| Tput (auc/s) | p50 | p99 | Tput (auc/s) | p50 | p99 |
| 5 | 75 | 77 | 5 | 30.9 | 31.2 |
| 10 | 67 | 72 | 20 | 29.8 | 31.5 |
| 15 | 56 | 72 | 25 | 24.9 | 29 |
| 18 | 73 | 86 | 30 | 20.5 | 25.5 |
| 20 | 125 | 156 | 50 | 16.4 | 29.5 |
| 30 | 378 | 637 | 60 | 26.7 | 35.2 |
| 50 | 930 | 1430 | 65 | 38 | 40 |
| | | | 70 | 78 | 117 |

## Expected Outputs

### Microbenchmarks (Figure 4)
- **CSV**: `obsidian/results/microbenchmark_results.csv`
- **Plots**: 4 PDF files in `obsidian/` directory

### Network Benchmarks (Figures 5, 7, 8, 9)
- **CSV files**: 
  - `obsidian/results/network_benchmark_*.csv`
  - `rel_work/addax/auction/auction-local-computation/results/addax_network_*.csv`
  - `rel_work/mp-spdz-0.3.9/results/mpspdz_network_*.csv`
- **Plots**: 4 PNG files in `rel_work/plots/`

### Throughput (Figure 10)
- **CSV**: `obsidian/throughput_results.csv`
- **Addax results**: Directories with throughput and latency text files
- **Plot**: `rel_work/plots/latency_vs_throughput.png`

## Artifact Description

### What is Included

This artifact provides:

1. **Complete Implementation**:
   - Obsidian: Full Rust implementation with all components (DPF, MPC, ring signatures)
   - Addax: Modified version for network and throughput benchmarks
   - MP-SPDZ: Configured for Vickrey auction benchmarks
   - SEAL: Homomorphic encryption implementation (excluded from main plots)

2. **Benchmarking Infrastructure**:
   - Automated scripts for running all experiments
   - Network latency emulation using `tc` (traffic control)
   - Result parsing and aggregation
   - Plotting scripts matching paper figures

3. **Reproducibility**:
   - Deterministic random seeds where applicable
   - Multiple runs with statistical aggregation
   - CSV output for all results
   - Automated plot generation

### What is NOT Included

- Pre-computed results (all experiments must be run)
- Pre-generated plots (plots are generated from fresh runs)
- Large pre-generated cryptographic materials (shares are generated on-demand)

### Key Features

- **Minimal Setup**: Most dependencies are included or have simple installation
- **Automated**: Single-command execution for each experiment
- **Modular**: Can run individual experiments independently
- **Transparent**: All scripts are readable and modifiable

### Verification

To verify the artifact works correctly:

1. **Quick Test** (5 minutes):
   ```bash
   cd obsidian
   ./run_microbenchmarks.sh 1  # Single run, quick test
   python3 plot_microbenchmarks.py
   ```
   Should generate 4 PDF plots.

2. **Network Test** (15 minutes):
   ```bash
   cd rel_work
   sudo ./run_required_benchmarks.sh 1  # Single run per config
   python3 plot_network_benchmarks.py
   ```
   Should generate 4 PNG plots.

3. **Throughput Test** (30 minutes):
   ```bash
   cd rel_work
   sudo ./run_throughput_benchmarks.sh
   # Wait for completion, then:
   python3 plot_throughput.py
   ```
   Should generate latency vs throughput plot.

### Troubleshooting

**Issue**: Network benchmarks fail with permission errors
- **Solution**: Ensure `sudo` access and that `tc` (traffic control) is available

**Issue**: Rust build fails
- **Solution**: Ensure Rust is installed via rustup and `RUSTFLAGS+="-C target-cpu=native"` is set

**Issue**: Python plots fail
- **Solution**: Install matplotlib: `pip3 install --user matplotlib`

**Issue**: Addax benchmarks fail
- **Solution**: Check that Addax binaries are built (see `rel_work/addax/README.md`)

**Issue**: MP-SPDZ benchmarks fail
- **Solution**: Check MP-SPDZ setup (see `rel_work/mpspdz-artifacts/README.md`)

### Contact

For questions or issues with the artifact, please refer to the paper or contact the authors.

---

**Last Updated**: December 2024
