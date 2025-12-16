#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

# Read results
df = pd.read_csv('throughput_results.csv')

# Create plots
fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(15, 12))

# Latency vs Throughput
ax1.plot(df['actual_aps'], df['p50_ms'], 'o-', label='P50', linewidth=2)
ax1.plot(df['actual_aps'], df['p99_ms'], 's-', label='P99', linewidth=2)
ax1.set_xlabel('Actual Throughput (auctions/sec)')
ax1.set_ylabel('Latency (ms)')
ax1.set_title('Latency vs Achieved Throughput')
ax1.legend()
ax1.grid(True, alpha=0.3)
ax1.set_yscale('log')

# Efficiency
efficiency = df['actual_aps'] / df['target_aps'] * 100
ax2.plot(df['target_aps'], efficiency, 'go-', linewidth=2)
ax2.axhline(y=100, color='r', linestyle='--', alpha=0.5, label='100% efficiency')
ax2.set_xlabel('Target Throughput (auctions/sec)')
ax2.set_ylabel('Efficiency (%)')
ax2.set_title('System Efficiency')
ax2.legend()
ax2.grid(True, alpha=0.3)

# Actual vs Target throughput
ax3.plot([0, df['target_aps'].max()], [0, df['target_aps'].max()], 'k--', alpha=0.5, label='Perfect scaling')
ax3.plot(df['target_aps'], df['actual_aps'], 'ro-', linewidth=2, label='Actual')
ax3.set_xlabel('Target Throughput (auctions/sec)')
ax3.set_ylabel('Actual Throughput (auctions/sec)')
ax3.set_title('Throughput Scaling')
ax3.legend()
ax3.grid(True, alpha=0.3)

# Sample count
ax4.bar(df['target_aps'], df['samples'], alpha=0.7)
ax4.set_xlabel('Target Throughput (auctions/sec)')
ax4.set_ylabel('Completed Auctions')
ax4.set_title('Sample Count per Test')
ax4.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig('single_core_analysis.png', dpi=300, bbox_inches='tight')
print("Analysis plots saved to: single_core_analysis.png")

# Find optimal operating point
print("\nKEY FINDINGS:")
print("-" * 40)

# Max efficient throughput (>90% efficiency)
efficient = df[efficiency >= 90]
if len(efficient) > 0:
    max_efficient = efficient['actual_aps'].max()
    print(f"Max efficient throughput (≥90%): {max_efficient:.1f} aps")

# Max throughput with reasonable latency (P99 < 100ms)
reasonable = df[df['p99_ms'] < 100]
if len(reasonable) > 0:
    max_reasonable = reasonable['actual_aps'].max()
    print(f"Max throughput with P99 < 100ms: {max_reasonable:.1f} aps")

# Overall max
print(f"Peak throughput achieved: {df['actual_aps'].max():.1f} aps")
print(f"Best P50 latency: {df['p50_ms'].min():.1f}ms")
print(f"Best P99 latency: {df['p99_ms'].min():.1f}ms")
