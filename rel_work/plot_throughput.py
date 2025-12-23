#!/usr/bin/env python3

"""
Script to generate throughput and latency comparison plots for Obsidian and Addax
"""

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import csv
import re
import os
import sys
import glob

# Font settings
plt.rcParams.update({
    'font.size': 16,
    'axes.titlesize': 16,
    'axes.labelsize': 16,
    'xtick.labelsize': 16,
    'ytick.labelsize': 16,
    'font.weight': 'normal',
})

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

def load_obsidian_throughput():
    """Load Obsidian throughput results from CSV"""
    csv_file = os.path.join(SCRIPT_DIR, '..', 'obsidian', 'throughput_results.csv')
    
    if not os.path.exists(csv_file):
        return [], []
    
    target_aps = []
    actual_aps = []
    
    try:
        with open(csv_file, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                target_aps.append(float(row['target_aps']))
                actual_aps.append(float(row['actual_aps']))
    except Exception as e:
        print(f"Error loading Obsidian results: {e}", file=sys.stderr)
    
    return target_aps, actual_aps

def load_obsidian_latency():
    """Load Obsidian latency data (p50, p99) from CSV"""
    csv_file = os.path.join(SCRIPT_DIR, '..', 'obsidian', 'throughput_results.csv')
    
    if not os.path.exists(csv_file):
        return [], [], []
    
    throughput = []
    p50 = []
    p99 = []
    
    try:
        with open(csv_file, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                throughput.append(float(row['actual_aps']))
                p50.append(float(row['p50_ms']))
                p99.append(float(row['p99_ms']))
    except Exception as e:
        print(f"Error loading Obsidian latency: {e}", file=sys.stderr)
    
    return throughput, p50, p99

def load_addax_throughput(results_dir):
    """Load Addax throughput results from text files"""
    if not os.path.exists(results_dir):
        return [], []
    
    # Pattern: throughput_<servers>_<auctions>.txt
    throughput_data = {}
    
    for txt_file in glob.glob(os.path.join(results_dir, 'throughput_*.txt')):
        try:
            # Extract config from filename
            basename = os.path.basename(txt_file)
            match = re.search(r'throughput_(\d+)_(\d+)\.txt', basename)
            if match:
                num_servers = int(match.group(1))
                num_auctions = int(match.group(2))
                
                # Read throughput value from file
                with open(txt_file, 'r') as f:
                    content = f.read()
                    # Look for "total throughput: <number>"
                    match_throughput = re.search(r'total throughput:\s*(\d+)', content)
                    if match_throughput:
                        throughput = int(match_throughput.group(1))
                        if num_auctions not in throughput_data:
                            throughput_data[num_auctions] = []
                        throughput_data[num_auctions].append(throughput)
        except Exception as e:
            print(f"Error loading {txt_file}: {e}", file=sys.stderr)
    
    # Average multiple server configurations for same auction count
    auctions = sorted(throughput_data.keys())
    throughputs = []
    for auction_count in auctions:
        avg_throughput = sum(throughput_data[auction_count]) / len(throughput_data[auction_count])
        throughputs.append(avg_throughput)
    
    return auctions, throughputs

def load_addax_latency(results_dir):
    """Load Addax latency results (p50, p99) from text files"""
    if not os.path.exists(results_dir):
        return [], [], []
    
    # Pattern: latency_<servers>_<auctions>.txt
    latency_data = {}
    
    for txt_file in glob.glob(os.path.join(results_dir, 'latency_*.txt')):
        try:
            # Extract config from filename
            basename = os.path.basename(txt_file)
            match = re.search(r'latency_(\d+)_(\d+)\.txt', basename)
            if match:
                num_servers = int(match.group(1))
                num_auctions = int(match.group(2))
                
                # Read latency values from file
                with open(txt_file, 'r') as f:
                    content = f.read()
                    # Look for "median latency: <number>" and "99% latency: <number>"
                    match_p50 = re.search(r'median latency:\s*([\d.]+)', content)
                    match_p99 = re.search(r'99% latency:\s*([\d.]+)', content)
                    
                    if match_p50 and match_p99:
                        p50_val = float(match_p50.group(1))
                        p99_val = float(match_p99.group(1))
                        
                        if num_auctions not in latency_data:
                            latency_data[num_auctions] = {'p50': [], 'p99': []}
                        latency_data[num_auctions]['p50'].append(p50_val)
                        latency_data[num_auctions]['p99'].append(p99_val)
        except Exception as e:
            print(f"Error loading {txt_file}: {e}", file=sys.stderr)
    
    # Average multiple server configurations for same auction count
    auctions = sorted(latency_data.keys())
    p50_list = []
    p99_list = []
    for auction_count in auctions:
        avg_p50 = sum(latency_data[auction_count]['p50']) / len(latency_data[auction_count]['p50'])
        avg_p99 = sum(latency_data[auction_count]['p99']) / len(latency_data[auction_count]['p99'])
        p50_list.append(avg_p50)
        p99_list.append(avg_p99)
    
    return auctions, p50_list, p99_list

def plot_latency_vs_throughput():
    """Generate latency vs throughput plot (main plot from paper)"""
    fig, ax = plt.subplots(1, 1, figsize=(10, 6))
    
    # Load Obsidian data
    obsidian_tput, obsidian_p50, obsidian_p99 = load_obsidian_latency()
    
    # Load Addax 2-round data
    addax_2round_dir = os.path.join(SCRIPT_DIR, 'addax', 'auction', 'throughput', 'results_2round')
    addax_2round_auctions, addax_2round_p50, addax_2round_p99 = load_addax_latency(addax_2round_dir)
    addax_2round_auctions_tput, addax_2round_throughput = load_addax_throughput(addax_2round_dir)
    
    # Load Addax non-interactive data
    addax_noninteractive_dir = os.path.join(SCRIPT_DIR, 'addax', 'auction', 'throughput', 'results_noninteractive')
    addax_noninteractive_auctions, addax_noninteractive_p50, addax_noninteractive_p99 = load_addax_latency(addax_noninteractive_dir)
    addax_noninteractive_auctions_tput, addax_noninteractive_throughput = load_addax_throughput(addax_noninteractive_dir)
    
    # Plot Obsidian (green, + markers)
    if obsidian_tput and obsidian_p50 and obsidian_p99:
        ax.plot(obsidian_tput, obsidian_p50, 'g*-', label='Obsidian p50', linewidth=2, markersize=8)
        ax.plot(obsidian_tput, obsidian_p99, 'g*--', label='Obsidian p99', linewidth=2, markersize=8)
    
    # Plot Addax 2-round (orange, + markers) - use throughput from throughput files, latency from latency files
    # Match up by auction count
    if addax_2round_auctions and addax_2round_p50:
        # Create a mapping from auction count to throughput
        tput_map = {}
        for i, auc in enumerate(addax_2round_auctions_tput):
            tput_map[auc] = addax_2round_throughput[i]
        
        # Get throughput for each latency measurement
        tput_for_latency = []
        p50_for_plot = []
        p99_for_plot = []
        for i, auc in enumerate(addax_2round_auctions):
            if auc in tput_map:
                tput_for_latency.append(tput_map[auc])
                p50_for_plot.append(addax_2round_p50[i])
                p99_for_plot.append(addax_2round_p99[i])
        
        if tput_for_latency:
            ax.plot(tput_for_latency, p50_for_plot, 'o-', label='Addax (2-Round) p50', linewidth=2, markersize=8, color='#ff7f0e')
            ax.plot(tput_for_latency, p99_for_plot, 'o--', label='Addax (2-Round) p99', linewidth=2, markersize=8, color='#ff7f0e')
    
    # Plot Addax non-interactive (blue, diamond markers)
    if addax_noninteractive_auctions and addax_noninteractive_p50:
        # Create a mapping from auction count to throughput
        tput_map = {}
        for i, auc in enumerate(addax_noninteractive_auctions_tput):
            tput_map[auc] = addax_noninteractive_throughput[i]
        
        # Get throughput for each latency measurement
        tput_for_latency = []
        p50_for_plot = []
        p99_for_plot = []
        for i, auc in enumerate(addax_noninteractive_auctions):
            if auc in tput_map:
                tput_for_latency.append(tput_map[auc])
                p50_for_plot.append(addax_noninteractive_p50[i])
                p99_for_plot.append(addax_noninteractive_p99[i])
        
        if tput_for_latency:
            ax.plot(tput_for_latency, p50_for_plot, 'D-', label='Addax (Non-Interactive) p50', linewidth=2, markersize=8, color='#1f77b4')
            ax.plot(tput_for_latency, p99_for_plot, 'D--', label='Addax (Non-Interactive) p99', linewidth=2, markersize=8, color='#1f77b4')
    
    ax.set_xlabel('Throughput (auctions/second)')
    ax.set_ylabel('Latency (ms)')
    ax.set_title('Latency (ms) vs. Throughput (auctions/second)')
    ax.legend(loc='upper left', fontsize=12)
    ax.grid(True, alpha=0.3, linestyle='-', linewidth=0.5)
    ax.set_xlim(left=0, right=500)
    ax.set_ylim(bottom=0, top=1000)
    
    plt.tight_layout()
    
    output_dir = os.path.join(SCRIPT_DIR, 'plots')
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, 'latency_vs_throughput.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()

def plot_throughput_comparison():
    """Generate throughput comparison plot"""
    fig, ax = plt.subplots(1, 1, figsize=(10, 6))
    
    # Load Obsidian data
    obsidian_target, obsidian_actual = load_obsidian_throughput()
    
    # Load Addax 2-round data
    addax_2round_dir = os.path.join(SCRIPT_DIR, 'addax', 'auction', 'throughput', 'results_2round')
    addax_2round_auctions, addax_2round_throughput = load_addax_throughput(addax_2round_dir)
    
    # Load Addax non-interactive data
    addax_noninteractive_dir = os.path.join(SCRIPT_DIR, 'addax', 'auction', 'throughput', 'results_noninteractive')
    addax_noninteractive_auctions, addax_noninteractive_throughput = load_addax_throughput(addax_noninteractive_dir)
    
    # Plot Obsidian
    if obsidian_target and obsidian_actual:
        ax.plot(obsidian_target, obsidian_actual, 'o-', label='Obsidian', linewidth=2, markersize=6, color='#2ca02c')
    
    # Plot Addax 2-round
    if addax_2round_auctions and addax_2round_throughput:
        ax.plot(addax_2round_auctions, addax_2round_throughput, 's-', label='Addax (2-round)', linewidth=2, markersize=6, color='#ff7f0e')
    
    # Plot Addax non-interactive
    if addax_noninteractive_auctions and addax_noninteractive_throughput:
        ax.plot(addax_noninteractive_auctions, addax_noninteractive_throughput, '^-', label='Addax (non-interactive)', linewidth=2, markersize=6, color='#1f77b4')
    
    ax.set_xlabel('Load (auctions)')
    ax.set_ylabel('Throughput (auctions/sec)')
    ax.set_title('Throughput Comparison')
    ax.legend()
    ax.grid(True, alpha=0.3)
    ax.set_xscale('log')
    ax.set_yscale('log')
    
    plt.tight_layout()
    
    output_dir = os.path.join(SCRIPT_DIR, 'plots')
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, 'throughput_comparison.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()

def main():
    """Generate throughput and latency plots"""
    print("Generating throughput and latency plots...")
    print("")
    
    plot_latency_vs_throughput()
    plot_throughput_comparison()
    
    print("")
    print("All plots generated in: plots/")
    print("Generated plots:")
    print("  - latency_vs_throughput.png (main plot)")
    print("  - throughput_comparison.png")

if __name__ == '__main__':
    main()
