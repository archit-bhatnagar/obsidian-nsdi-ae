#!/usr/bin/env python3
import os
import numpy as np
import matplotlib.pyplot as plt
import pandas as pd
from pathlib import Path
import seaborn as sns

def analyze_latencies(file_path):
    """Analyze latencies from a file and return statistics."""
    try:
        latencies = np.loadtxt(file_path)
        if len(latencies) == 0:
            return None
        
        return {
            'count': len(latencies),
            'mean': np.mean(latencies),
            'std': np.std(latencies),
            'p50': np.percentile(latencies, 50),
            'p95': np.percentile(latencies, 95),
            'p99': np.percentile(latencies, 99),
            'p999': np.percentile(latencies, 99.9),
            'min': np.min(latencies),
            'max': np.max(latencies)
        }
    except Exception as e:
        print(f"Error reading {file_path}: {e}")
        return None

def main():
    results_dir = Path('.')
    results = []
    
    # Process all latency files
    for file_path in results_dir.glob('latencies_*_aps.txt'):
        filename = file_path.stem
        throughput_str = filename.replace('latencies_', '').replace('_aps', '')
        throughput = int(throughput_str)
        
        stats = analyze_latencies(file_path)
        if stats:
            stats['throughput'] = throughput
            results.append(stats)
    
    if not results:
        print("No valid results found!")
        return
    
    # Sort by throughput
    results.sort(key=lambda x: x['throughput'])
    df = pd.DataFrame(results)
    
    # Print summary table
    print("40-CORE THROUGHPUT BENCHMARK RESULTS")
    print("=" * 100)
    print(f"{'Throughput':<12} {'Count':<8} {'P50':<8} {'P95':<8} {'P99':<8} {'P99.9':<8} {'Mean':<10} {'Max':<10}")
    print(f"{'(aps)':<12} {'(#)':<8} {'(ms)':<8} {'(ms)':<8} {'(ms)':<8} {'(ms)':<8} {'(ms)':<10} {'(ms)':<10}")
    print("-" * 100)
    
    for _, row in df.iterrows():
        print(f"{row['throughput']:<12d} {row['count']:<8d} {row['p50']:<8.2f} {row['p95']:<8.2f} {row['p99']:<8.2f} {row['p999']:<8.2f} {row['mean']:<10.2f} {row['max']:<10.2f}")
    
    # Save CSV
    df.to_csv('benchmark_summary_40core.csv', index=False)
    print(f"\nDetailed results saved to: benchmark_summary_40core.csv")
    
    # Create comprehensive plots
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(16, 12))
    
    # P50, P95, P99 vs Throughput
    ax1.plot(df['throughput'], df['p50'], 'o-', label='P50', linewidth=2, markersize=4)
    ax1.plot(df['throughput'], df['p95'], 's-', label='P95', linewidth=2, markersize=4)
    ax1.plot(df['throughput'], df['p99'], '^-', label='P99', linewidth=2, markersize=4)
    ax1.plot(df['throughput'], df['p999'], 'v-', label='P99.9', linewidth=2, markersize=4)
    ax1.set_xlabel('Target Throughput (auctions/second)')
    ax1.set_ylabel('Latency (ms)')
    ax1.set_title('Latency Percentiles vs Throughput (40-Core System)')
    ax1.legend()
    ax1.grid(True, alpha=0.3)
    ax1.set_xscale('log')
    ax1.set_yscale('log')
    
    # Throughput achieved vs target
    actual_throughput = df['count'] / 60  # Assuming 60 second test duration
    ax2.plot([0, df['throughput'].max()], [0, df['throughput'].max()], 'k--', alpha=0.5, label='Perfect scaling')
    ax2.plot(df['throughput'], actual_throughput, 'ro-', linewidth=2, markersize=6, label='Actual throughput')
    ax2.set_xlabel('Target Throughput (auctions/second)')
    ax2.set_ylabel('Actual Throughput (auctions/second)')
    ax2.set_title('Throughput Scaling on 40-Core System')
    ax2.legend()
    ax2.grid(True, alpha=0.3)
    
    # Latency distribution heatmap for high throughput cases
    high_throughput_cases = df[df['throughput'] >= 40].head(6)
    if len(high_throughput_cases) > 0:
        latency_data = []
        labels = []
        for _, row in high_throughput_cases.iterrows():
            latency_file = f'latencies_{int(row["throughput"])}_aps.txt'
            if os.path.exists(latency_file):
                latencies = np.loadtxt(latency_file)
                if len(latencies) > 100:  # Only if we have enough samples
                    latency_data.append(latencies[:1000])  # Limit to 1000 samples for visualization
                    labels.append(f'{int(row["throughput"])} aps')
        
        if latency_data:
            ax3.violinplot(latency_data, positions=range(len(latency_data)), showmeans=True)
            ax3.set_xticks(range(len(labels)))
            ax3.set_xticklabels(labels, rotation=45)
            ax3.set_ylabel('Latency (ms)')
            ax3.set_title('Latency Distribution at High Throughput')
            ax3.grid(True, alpha=0.3)
    
    # System efficiency metrics
    efficiency = actual_throughput / df['throughput'] * 100
    ax4.plot(df['throughput'], efficiency, 'go-', linewidth=2, markersize=6)
    ax4.axhline(y=100, color='k', linestyle='--', alpha=0.5, label='100% efficiency')
    ax4.set_xlabel('Target Throughput (auctions/second)')
    ax4.set_ylabel('Efficiency (%)')
    ax4.set_title('System Efficiency vs Throughput')
    ax4.legend()
    ax4.grid(True, alpha=0.3)
    ax4.set_ylim(0, 120)
    
    plt.tight_layout()
    plt.savefig('40core_latency_analysis.png', dpi=300, bbox_inches='tight')
    print("Plots saved to: 40core_latency_analysis.png")
    
    # Print key insights
    print("\n" + "="*50)
    print("KEY INSIGHTS")
    print("="*50)
    max_efficient_throughput = df.loc[efficiency.idxmax()]
    print(f"Most efficient throughput: {max_efficient_throughput['throughput']} aps ({efficiency.max():.1f}% efficiency)")
    
    p99_under_threshold = df[df['p99'] < 1000]  # P99 under 1 second
    if len(p99_under_threshold) > 0:
        max_good_throughput = p99_under_threshold['throughput'].max()
        print(f"Maximum throughput with P99 < 1000ms: {max_good_throughput} aps")
    
    print(f"Peak throughput tested: {df['throughput'].max()} aps")
    print(f"Best P99 latency: {df['p99'].min():.2f}ms at {df.loc[df['p99'].idxmin(), 'throughput']} aps")

if __name__ == "__main__":
    main()
