# Obsidian: Private Vickrey Auctions

Implementation of Obsidian, a system for private Vickrey auctions using secure multi-party computation.

**WARNING: This is research prototype code, not production-ready.**

## Building

```bash
export RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native"
cargo build --release
```

## Running Microbenchmarks

To replicate the microbenchmark results from the paper:

1. **Run benchmarks:**
   ```bash
   ./run_microbenchmarks.sh
   ```
   This runs benchmarks across different configurations and saves results to `results/microbenchmark_results.csv`.

2. **Generate plots:**
   ```bash
   python3 plot_microbenchmarks.py
   ```
   Requires matplotlib: `python3 -m pip install --user matplotlib`

   Generates:
   - `scale_domain.pdf` - Timing vs domain (100 bidders)
   - `scale_bidders.pdf` - Timing vs bidders (1024 domain)
   - `communication_domain.pdf` - Communication vs domain (100 bidders)
   - `communication_bidders.pdf` - Communication vs bidders (1024 domain)

## Other Binaries

- `dpf_benchmark` - Microbenchmark with timing and communication tracking
- `dpf_run` - Basic auction protocol execution
- `dpf_run_comm` - Execution with detailed communication analysis

## Network Benchmarks

To run network benchmarks with RTT emulation:

```bash
sudo ./run_network_benchmark.sh <num_bidders> <domain_size> <rtt_ms> [num_runs]
```

Results are saved to `results/network_benchmark_<bidders>_<domain>_<rtt>ms.csv`.

## Results

All benchmark results are saved in `results/`:
- `microbenchmark_results.csv` - Detailed per-run data (timing + communication)
- `microbenchmark_raw.txt` - Raw benchmark output
- `network_benchmark_*.csv` - Network benchmark results with RTT emulation
