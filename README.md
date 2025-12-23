# Secure Vickrey Auctions for Online Advertising

## Overview

The artifact provides the implementation of Obsidian, a system for secure Vickrey auctions with bid confidentiality and bidder anonymity, along with implementations of related work systems (Addax, MP-SPDZ, SEAL) for a comparative evaluation. 
This artifact is relatively less mature and currently not tested on different platforms except Ubuntu 20.04, and we'd be happy to troubleshoot for any foreseeable errors with Obsidian and the related works. 
Obsidian code is written in Rust (cargo packages should ideally build automatically when running the files), and other related work is a combination of Python and C++.

We are applying for all 3 badges.

## What's Included

**Complete Implementation:**
- Obsidian: Full Rust implementation with DPF, MPC, and ring signature components
- Related Works: Addax (Secure Auction using Prio encodings), MP-SPDZ (Multi-party computation), SEAL (Homomorphic encryption)

**Benchmarking Setup:**
- Automated scripts for all experiments, emulation, parsing, and plotting

**Experiments:**

- Figure 4: Microbenchmarks (offline/online phases, varying bidders and domain sizes)
- Figures 5, 7, 8, 9: Network benchmarks comparing latency and communication across Obsidian, Addax, and MP-SPDZ. We skip SEAL as it takes significantly longer to run (often exceeding an hour) and performs much worse than other systems. The SEAL code is available in `rel_work/seal-auction/` for separate evaluation if desired.
- Figure 10: Throughput and latency analysis for Obsidian and Addax protocols


## System Requirements

- OS: Linux (tested on Ubuntu 20.04+)
- Memory: 4GB minimum
- Dependencies: Rust (via rustup), Python 3 with matplotlib, standard build tools
- Privileges: sudo access required for network latency emulation

## Expected Runtime

- Microbenchmarks (Fig 4): ~10-15 minutes
- Network Benchmarks (Figs 5, 7, 8, 9): ~30-45 minutes (required configs for the plot)
- Throughput (Fig 10): ~20 minutes

## Quick Start

1. Build Obsidian: `cd obsidian && cargo build --release`
2. Run microbenchmarks: `./run_microbenchmarks.sh && python3 plot_microbenchmarks.py`
3. Run network benchmarks: `cd ../rel_work && sudo ./run_network_benchmarks.sh && python3 plot_network_benchmarks.py`
4. Run throughput: `sudo ./run_throughput_benchmarks.sh && python3 plot_throughput.py`

## Artifact Structure

```
obsidian-nsdi/
├── README.md                    # Detailed artifact evaluation guide
├── obsidian/                    # Obsidian implementation
│   ├── src/                     # Rust source code
│   ├── run_microbenchmarks.sh  # Fig 4 experiments
│   └── plot_microbenchmarks.py  # Fig 4 plots
└── rel_work/                    # Related work comparison
    ├── addax/                   # Addax implementation
    ├── mpspdz-artifacts/          # MP-SPDZ implementation
    ├── seal-auction/            # SEAL implementation
    ├── run_network_benchmarks.sh    # Figs 5,7,8,9 experiments
    ├── run_throughput_benchmarks.sh  # Fig 10 experiments
    ├── plot_network_benchmarks.py    # Figs 5,7,8,9 plots
    └── plot_throughput.py            # Fig 10 plots
```

All experiments generate plots matching the paper figures. The README includes detailed step-by-step instructions.

## Detailed Instructions

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

---

### Figures 5, 7, 8, 9: Network Benchmarks (Latency & Communication)

**Description**: Compares Obsidian, Addax, and MP-SPDZ on latency and communication under different network conditions (20ms and 40ms RTT).

**Note**: SEAL is excluded from these plots due to extremely long runtimes and poor performance. The SEAL code is available in `rel_work/seal-auction/` for separate evaluation.

**Configurations**:
- **Varying bidders** (domain fixed at 1000): 25, 50, 100 bidders, RTT: 20ms, 40ms
- **Varying domain** (bidders fixed at 100): 100, 1000, 10000 domains, RTT: 20ms, 40ms

**Steps**:

1. **Run network benchmarks**:
   ```bash
   cd rel_work
   sudo ./run_network_benchmarks.sh [num_runs]
   ```
   - Default: 3 runs per configuration
   - Runs the 10 unique configurations needed for the 4 plots

2. **Generate plots**:
   ```bash
   python3 plot_network_benchmarks.py
   ```
   - Generates 4 PNG files in `rel_work/plots/`:
     - `comm_vs_bidders.png` - Communication vs Number of Bidders (Fig 7)
     - `comm_vs_domain.png` - Communication vs Domain Size (Fig 8)
     - `time_vs_bidders.png` - Time vs Number of Bidders with 20ms/40ms RTT (Fig 5)
     - `time_vs_domain.png` - Time vs Domain Size with 20ms/40ms RTT (Fig 9)

**Expected Runtime**: ~30-45 minutes

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

2. **Generate plot**:
   ```bash
   python3 plot_throughput.py
   ```
   - Generates `latency_vs_throughput.png` in `rel_work/plots/`
   - Shows p50 and p99 latency vs throughput for all three systems

**Expected Runtime**: ~20 minutes

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

## Troubleshooting

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

## Contact

For questions or issues with the artifact, please refer to the paper or contact the authors.

---

**Last Updated**: December 2024
