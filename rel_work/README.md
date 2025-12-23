# Network Benchmarking Suite

This directory contains scripts to run network benchmarks across multiple systems and generate comparison plots.

**Note:** The plots compare 3 systems: **Obsidian**, **Addax**, and **MP-SPDZ**. SEAL is excluded from the plots because it takes significantly longer to run (often exceeding hours for larger configurations), but the SEAL benchmark code is available in `seal-auction/` for evaluators who wish to run it separately.

## Running Benchmarks

### Run Network Benchmarks

To run the network benchmarks needed for the plots:

```bash
sudo ./run_network_benchmarks.sh [num_runs]
```

This will:
1. Run Obsidian benchmarks for all configurations
2. Run Addax benchmarks for all configurations
3. Run MP-SPDZ benchmarks for all configurations
4. Save results to CSV files in each system's results directory

**Configurations for plots:**
- **Varying bidders plots**: Domain fixed at 1000, bidders: 25, 50, 100, RTT: 20ms, 40ms
- **Varying domain plots**: Bidders fixed at 100, domains: 100, 1000, 10000, RTT: 20ms, 40ms

This runs the 10 unique configurations needed for the 4 plots.

## Generating Plots

After running benchmarks, generate comparison plots:

```bash
python3 plot_network_benchmarks.py
```

This generates 4 plots:
- `comm_vs_bidders.png` - Communication vs Number of Bidders (domain fixed at 1000)
- `comm_vs_domain.png` - Communication vs Domain Size (bidders fixed at 100)
- `time_vs_bidders.png` - Time vs Number of Bidders with 20ms and 40ms RTT (domain fixed at 1000)
- `time_vs_domain.png` - Time vs Domain Size with 20ms and 40ms RTT (bidders fixed at 100)

Each plot shows all 3 systems (Obsidian, Addax, MP-SPDZ). Time plots use solid bars for 20ms RTT and striped bars for 40ms RTT.

## Results Locations

- **Obsidian**: `obsidian/results/network_benchmark_*.csv`
- **Addax**: `addax/auction/auction-local-computation/results/addax_network_*.csv`
- **MP-SPDZ**: `mpspdz-artifacts/results/mpspdz_network_*.csv`

## Throughput Benchmarks

To run throughput benchmarks for Obsidian and Addax:

```bash
sudo ./run_throughput_benchmarks.sh
```

This will:
1. Run Obsidian throughput benchmark (single-core, varying load)
2. Run Addax 2-round throughput benchmarks (different server/client configurations)
3. Run Addax interactive throughput benchmarks (different server/client configurations)

Results are saved to:
- **Obsidian**: `obsidian/throughput_results.csv`
- **Addax 2-round**: `addax/auction/throughput/results_2round/`
- **Addax interactive**: `addax/auction/throughput/results_interactive/`

To generate throughput comparison plots:

```bash
python3 plot_throughput.py
```

This generates:
- `throughput_comparison.png` - Comparison of Obsidian, Addax 2-round, and Addax interactive

**Note:** For Addax throughput benchmarks, ensure shares are generated:
- For 2-round: `bash adv-gen-sc.sh <bidders> <buckets> <shares_dir> <commits_dir>`
- For interactive: `bash adv-gen-sc-interactive.sh <bidders> <buckets> <shares_dir> <commits_dir> <rounds>`

## Dependencies

- Python 3 with matplotlib
- All benchmark scripts must be set up (see individual system READMEs)

