#!/usr/bin/env python3

"""
Script to generate bar plots for network benchmark results
Generates 4 plots:
1. Communication vs Number of Bidders (for all systems)
2. Communication vs Domain Size (for all systems)
3. Time vs Number of Bidders (20ms and 40ms RTT for all systems)
4. Time vs Domain Size (20ms and 40ms RTT for all systems)
"""

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import csv
import numpy as np
import os
import sys

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

# Systems and their result directories
SYSTEMS = {
    'Obsidian': {
        'dir': os.path.join(SCRIPT_DIR, '..', 'obsidian', 'results'),
        'pattern': 'network_benchmark_{bidders}_{domain}_{rtt}ms.csv',
        'time_col': 'online_time_ms',
        'comm_col': 'online_comm_bytes',
        'color': '#2ca02c',  # Green
    },
    'Addax': {
        'dir': os.path.join(SCRIPT_DIR, 'addax', 'auction', 'auction-local-computation', 'results'),
        'pattern': 'addax_network_{bidders}_{domain}_{rtt}ms.csv',
        'time_col': 'online_time_s',
        'comm_col': 'comm_bytes_kb',
        'color': '#1f77b4',  # Blue
    },
    'MP-SPDZ': {
        'dir': os.path.join(SCRIPT_DIR, 'mp-spdz-0.3.9', 'results'),
        'pattern': 'mpspdz_network_{bidders}_{rtt}ms.csv',
        'time_col': 'online_time_s',
        'comm_col': 'comm_bytes_kb',
        'color': '#ff7f0e',  # Orange
    },
}

# Configurations to plot
RTT_VALUES = [20, 40]
DOMAIN_SIZES = [100, 1000, 10000]
BIDDER_COUNTS = [25, 50, 100]

# Fixed values for plots
FIXED_DOMAIN_FOR_BIDDERS = 1000  # Domain fixed at 1000 for varying bidders plots
FIXED_BIDDERS_FOR_DOMAIN = 100   # Bidders fixed at 100 for varying domain plots

def load_results(system_name, system_info, num_bidders, domain_size, rtt_ms):
    """Load and average results for a specific configuration"""
    if system_name == 'MP-SPDZ':
        # MP-SPDZ doesn't use domain_size in filename
        pattern = system_info['pattern'].format(bidders=num_bidders, rtt=rtt_ms)
    else:
        pattern = system_info['pattern'].format(bidders=num_bidders, domain=domain_size, rtt=rtt_ms)
    
    filepath = os.path.join(system_info['dir'], pattern)
    
    if not os.path.exists(filepath):
        return None, None
    
    try:
        # Read CSV manually (no pandas dependency)
        with open(filepath, 'r') as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        
        if len(rows) == 0:
            return None, None
        
        # Average across runs
        time_col = system_info['time_col']
        comm_col = system_info['comm_col']
        
        times = [float(row[time_col]) for row in rows if time_col in row and row[time_col]]
        comms = [float(row[comm_col]) for row in rows if comm_col in row and row[comm_col]]
        
        if len(times) == 0 or len(comms) == 0:
            return None, None
        
        avg_time = sum(times) / len(times)
        avg_comm = sum(comms) / len(comms)
        
        # Convert time to ms if needed
        if 's' in time_col and 'ms' not in time_col:
            avg_time = avg_time * 1000  # Convert seconds to ms
        
        # Convert communication to KB if needed
        if 'mb' in comm_col.lower():
            avg_comm = avg_comm * 1024  # Convert MB to KB
        elif 'bytes' in comm_col.lower() and 'kb' not in comm_col.lower() and 'mb' not in comm_col.lower():
            avg_comm = avg_comm / 1024  # Convert bytes to KB
        
        return avg_time, avg_comm
    except Exception as e:
        print(f"Error loading {filepath}: {e}", file=sys.stderr)
        return None, None

def plot_comm_vs_bidders():
    """Plot 1: Communication vs Number of Bidders (domain fixed at 1000)"""
    fig, ax = plt.subplots(1, 1, figsize=(10, 6))
    
    domain_size = FIXED_DOMAIN_FOR_BIDDERS
    x_pos = np.arange(len(BIDDER_COUNTS))
    width = 0.2
    
    for sys_idx, (system_name, system_info) in enumerate(SYSTEMS.items()):
        values = []
        for num_bidders in BIDDER_COUNTS:
            # Use 20ms RTT for communication (or any RTT, comm should be similar)
            _, value = load_results(system_name, system_info, num_bidders, domain_size, 20)
            
            if value is not None and value > 0:
                values.append(value)
            else:
                values.append(0)
        
        offset = (sys_idx - len(SYSTEMS)/2 + 0.5) * width
        ax.bar(x_pos + offset, values, width, label=system_name, color=system_info['color'])
    
    ax.set_xlabel('Number of Bidders')
    ax.set_ylabel('Communication (KB)')
    ax.set_title(f'Communication vs Number of Bidders (Domain: {domain_size})')
    ax.set_xticks(x_pos)
    ax.set_xticklabels(BIDDER_COUNTS)
    ax.legend()
    ax.grid(True, alpha=0.3, axis='y')
    ax.set_yscale('log')
    
    plt.tight_layout()
    
    output_dir = os.path.join(SCRIPT_DIR, 'plots')
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, 'comm_vs_bidders.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()

def plot_comm_vs_domain():
    """Plot 2: Communication vs Domain Size (bidders fixed at 100)"""
    fig, ax = plt.subplots(1, 1, figsize=(10, 6))
    
    num_bidders = FIXED_BIDDERS_FOR_DOMAIN
    x_pos = np.arange(len(DOMAIN_SIZES))
    width = 0.2
    
    for sys_idx, (system_name, system_info) in enumerate(SYSTEMS.items()):
        values = []
        for domain_size in DOMAIN_SIZES:
            # Use 20ms RTT for communication
            _, value = load_results(system_name, system_info, num_bidders, domain_size, 20)
            
            if value is not None and value > 0:
                values.append(value)
            else:
                values.append(0)
        
        offset = (sys_idx - len(SYSTEMS)/2 + 0.5) * width
        ax.bar(x_pos + offset, values, width, label=system_name, color=system_info['color'])
    
    ax.set_xlabel('Domain Size')
    ax.set_ylabel('Communication (KB)')
    ax.set_title(f'Communication vs Domain Size (Bidders: {num_bidders})')
    ax.set_xticks(x_pos)
    ax.set_xticklabels(DOMAIN_SIZES)
    ax.legend()
    ax.grid(True, alpha=0.3, axis='y')
    ax.set_yscale('log')
    
    plt.tight_layout()
    
    output_dir = os.path.join(SCRIPT_DIR, 'plots')
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, 'comm_vs_domain.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()

def plot_time_vs_bidders():
    """Plot 3: Time vs Number of Bidders (domain fixed at 1000, with 20ms and 40ms RTT)"""
    fig, ax = plt.subplots(1, 1, figsize=(10, 6))
    
    domain_size = FIXED_DOMAIN_FOR_BIDDERS
    x_pos = np.arange(len(BIDDER_COUNTS))
    width = 0.1  # Narrower bars since we have 2 RTTs × 4 systems = 8 bars per bidder count
    
    bar_idx = 0
    for sys_idx, (system_name, system_info) in enumerate(SYSTEMS.items()):
        for rtt_idx, rtt_ms in enumerate(RTT_VALUES):
            values = []
            for num_bidders in BIDDER_COUNTS:
                value, _ = load_results(system_name, system_info, num_bidders, domain_size, rtt_ms)
                
                if value is not None and value > 0:
                    values.append(value)
                else:
                    values.append(0)
            
            # Create pattern for 40ms (striped) vs solid for 20ms
            hatch = None if rtt_ms == 20 else '///'
            label = f'{system_name} ({rtt_ms}ms)'
            
            offset = (bar_idx - (len(SYSTEMS) * len(RTT_VALUES))/2 + 0.5) * width
            ax.bar(x_pos + offset, values, width, label=label, 
                  color=system_info['color'], hatch=hatch, alpha=0.8 if rtt_ms == 40 else 1.0)
            bar_idx += 1
    
    ax.set_xlabel('Number of Bidders')
    ax.set_ylabel('Time (ms)')
    ax.set_title(f'Time vs Number of Bidders (Domain: {domain_size})')
    ax.set_xticks(x_pos)
    ax.set_xticklabels(BIDDER_COUNTS)
    ax.legend(loc='upper left', fontsize=10, ncol=2)
    ax.grid(True, alpha=0.3, axis='y')
    ax.set_yscale('log')
    
    plt.tight_layout()
    
    output_dir = os.path.join(SCRIPT_DIR, 'plots')
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, 'time_vs_bidders.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()

def plot_time_vs_domain():
    """Plot 4: Time vs Domain Size (bidders fixed at 100, with 20ms and 40ms RTT)"""
    fig, ax = plt.subplots(1, 1, figsize=(10, 6))
    
    num_bidders = FIXED_BIDDERS_FOR_DOMAIN
    x_pos = np.arange(len(DOMAIN_SIZES))
    width = 0.1  # Narrower bars since we have 2 RTTs × 4 systems = 8 bars per domain
    
    bar_idx = 0
    for sys_idx, (system_name, system_info) in enumerate(SYSTEMS.items()):
        for rtt_idx, rtt_ms in enumerate(RTT_VALUES):
            values = []
            for domain_size in DOMAIN_SIZES:
                value, _ = load_results(system_name, system_info, num_bidders, domain_size, rtt_ms)
                
                if value is not None and value > 0:
                    values.append(value)
                else:
                    values.append(0)
            
            # Create pattern for 40ms (striped) vs solid for 20ms
            hatch = None if rtt_ms == 20 else '///'
            label = f'{system_name} ({rtt_ms}ms)'
            
            offset = (bar_idx - (len(SYSTEMS) * len(RTT_VALUES))/2 + 0.5) * width
            ax.bar(x_pos + offset, values, width, label=label,
                  color=system_info['color'], hatch=hatch, alpha=0.8 if rtt_ms == 40 else 1.0)
            bar_idx += 1
    
    ax.set_xlabel('Domain Size')
    ax.set_ylabel('Time (ms)')
    ax.set_title(f'Time vs Domain Size (Bidders: {num_bidders})')
    ax.set_xticks(x_pos)
    ax.set_xticklabels(DOMAIN_SIZES)
    ax.legend(loc='upper left', fontsize=10, ncol=2)
    ax.grid(True, alpha=0.3, axis='y')
    ax.set_yscale('log')
    
    plt.tight_layout()
    
    output_dir = os.path.join(SCRIPT_DIR, 'plots')
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, 'time_vs_domain.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()

def main():
    """Generate all plots"""
    print("Generating network benchmark plots...")
    print("")
    
    print("Plot 1: Communication vs Number of Bidders")
    plot_comm_vs_bidders()
    
    print("Plot 2: Communication vs Domain Size")
    plot_comm_vs_domain()
    
    print("Plot 3: Time vs Number of Bidders (20ms and 40ms RTT)")
    plot_time_vs_bidders()
    
    print("Plot 4: Time vs Domain Size (20ms and 40ms RTT)")
    plot_time_vs_domain()
    
    print("")
    print("All plots generated in: plots/")
    print("")
    print("Generated plots:")
    print("  - comm_vs_bidders.png")
    print("  - comm_vs_domain.png")
    print("  - time_vs_bidders.png")
    print("  - time_vs_domain.png")

if __name__ == '__main__':
    main()
