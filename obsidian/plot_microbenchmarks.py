#!/usr/bin/env python3
"""Plot microbenchmark results from CSV file."""

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.ticker as tick
import matplotlib.dates as mdates
import matplotlib.gridspec as gridspec
from matplotlib import rcParams
import csv
import statistics
import sys
import os

rcParams.update({'figure.autolayout': True})
rcParams.update({'errorbar.capsize': 2})
matplotlib.rcParams['pdf.fonttype'] = 42
matplotlib.rcParams['ps.fonttype'] = 42

font = {'weight': 'medium', 'size': 20}
matplotlib.rc('font', **font)

colors = [
    'tab:blue', 'tab:orange', 'tab:green', 'tab:red', 'tab:purple',
    'tab:brown', 'tab:pink', 'tab:gray', 'tab:olive', 'tab:cyan', 'black'
]

def format_number(y, pos=None):
    if y == 0.0001:
        return '0.0001'
    elif y == 0.001:
        return '0.001'
    elif y == 0.01:
        return '0.01'
    elif 1000**2 <= y < 1000**3:
        return '%dM' % (y/1000000)
    elif 1000**3 <= y:
        return '%dG' % (y/1000000000)
    else:
        return '%d' % (y)

def load_results(csv_file):
    """Load timing results from CSV."""
    results = {}
    with open(csv_file, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            key = (int(row['num_bidders']), int(row['domain_size']), row['phase'])
            if key not in results:
                results[key] = []
            results[key].append(float(row['time_ms']))
    return results

def load_comm_results(csv_file):
    """Load communication results from CSV."""
    comm_results = {}
    with open(csv_file, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            key = (int(row['num_bidders']), int(row['domain_size']), row['phase'])
            if key not in comm_results:
                comm_results[key] = []
            if row['comm_bytes']:
                comm_results[key].append(int(row['comm_bytes']))
    return comm_results

# ------------------------- Scale Bidders Communication ------------------------- #
def plot_comm_bidders(comm_results, output_dir="results"):
    plt.clf()
    fig, ax = plt.subplots(figsize=(6.4, 4.8))
    plt.grid(True)
    
    num_of_bidders = [25, 50, 100, 200, 400, 800]
    comm_bytes = []
    preproc_comm = []
    
    for bidders in num_of_bidders:
        key_online = (bidders, 1024, 'online')
        key_preprocess = (bidders, 1024, 'preprocess')
        
        if key_online in comm_results and len(comm_results[key_online]) > 0:
            comm_bytes.append(statistics.mean(comm_results[key_online]))
        else:
            comm_bytes.append(0)
        
        if key_preprocess in comm_results and len(comm_results[key_preprocess]) > 0:
            preproc_comm.append(statistics.mean(comm_results[key_preprocess]))
        else:
            preproc_comm.append(0)
    
    # Filter to only include bidders with data
    filtered = [(b, c, p) for b, c, p in zip(num_of_bidders, comm_bytes, preproc_comm) if c > 0 and p > 0]
    if not filtered:
        print("No data for comm_bidders plot")
        return
    
    num_of_bidders, comm_bytes, preproc_comm = zip(*filtered)
    
    comm_KB = [x / 1024 for x in comm_bytes]
    preproc_comm_KB = [x / 1024 for x in preproc_comm]
    
    plt.plot(num_of_bidders, comm_KB, marker='o', linestyle='-', color=colors[0], label='Online Phase')
    plt.plot(num_of_bidders, preproc_comm_KB, marker='s', linestyle='--', color=colors[1], label='Preprocessing Phase')
    plt.xscale('log', base=2)
    ax.set_xticks(list(num_of_bidders))
    ax.set_xticklabels(list(num_of_bidders))
    ax.xaxis.set_major_formatter(tick.FuncFormatter(format_number))
    ax.set_xlim(right=1000)
    ax.set_ylim(0, 135)
    ax.yaxis.set_major_formatter(tick.FuncFormatter(format_number))
    plt.xlabel('Number of Bidders')
    plt.ylabel('Communication Cost (KB)')
    plt.grid(True)
    plt.xticks(num_of_bidders)
    plt.legend(loc='center left')
    plt.tight_layout()
    output_file = os.path.join(output_dir, "comm_cost_bidders.pdf")
    plt.savefig(output_file)
    print(f"Saved {output_file}")

# ------------------------- Scale Bidders ------------------------- #
def plot_scale_bidders(results, output_dir="results"):
    plt.clf()
    fig, ax = plt.subplots(figsize=(6.4, 4.8))
    plt.grid(True)
    
    num_of_bidders = [25, 50, 100, 200, 400, 800]
    online_time = []
    preprocessing_time = []
    
    for bidders in num_of_bidders:
        key_online = (bidders, 1024, 'online')
        key_preprocess = (bidders, 1024, 'preprocess')
        
        if key_online in results and len(results[key_online]) > 0:
            online_time.append(statistics.mean(results[key_online]))
        else:
            online_time.append(0)
        
        if key_preprocess in results and len(results[key_preprocess]) > 0:
            preprocessing_time.append(statistics.mean(results[key_preprocess]))
        else:
            preprocessing_time.append(0)
    
    filtered = [(b, o, p) for b, o, p in zip(num_of_bidders, online_time, preprocessing_time) if o > 0 and p > 0]
    if not filtered:
        print("No data for scale_bidders plot")
        return
    
    num_of_bidders, online_time, preprocessing_time = zip(*filtered)
    
    plt.plot(num_of_bidders, online_time, marker='o', linestyle='-', color=colors[0], label='Online Phase')
    plt.plot(num_of_bidders, preprocessing_time, marker='s', linestyle='--', color=colors[1], label='Preprocessing Phase')
    plt.xscale('log', base=2)
    ax.set_xticks(list(num_of_bidders))
    ax.set_xticklabels(list(num_of_bidders))
    ax.xaxis.set_major_formatter(tick.FuncFormatter(format_number))
    ax.set_yscale('log')
    ax.set_ylim(0.6, 900)
    ax.yaxis.set_major_formatter(tick.FuncFormatter(format_number))
    plt.xlabel('Number of Bidders')
    plt.ylabel('Time (ms)')
    plt.grid(True)
    plt.xticks(num_of_bidders)
    plt.legend(loc=(0.04, 0.51))
    plt.tight_layout()
    output_file = os.path.join(output_dir, "scale_bidders.pdf")
    plt.savefig(output_file)
    print(f"Saved {output_file}")

# ------------------------- Scale Domain Communication ------------------------- #
def plot_comm_domain(comm_results, output_dir="results"):
    plt.clf()
    fig, ax = plt.subplots(figsize=(6.4, 4.8))
    plt.grid(True)
    
    domain = [128, 256, 512, 1024, 2048, 4096]
    comm_bytes = []
    preproc_comm = []
    
    for d in domain:
        key_online = (100, d, 'online')
        key_preprocess = (100, d, 'preprocess')
        
        if key_online in comm_results and len(comm_results[key_online]) > 0:
            comm_bytes.append(statistics.mean(comm_results[key_online]))
        else:
            comm_bytes.append(0)
        
        if key_preprocess in comm_results and len(comm_results[key_preprocess]) > 0:
            preproc_comm.append(statistics.mean(comm_results[key_preprocess]))
        else:
            preproc_comm.append(0)
    
    # Filter to only include domains with data
    filtered = [(d, c, p) for d, c, p in zip(domain, comm_bytes, preproc_comm) if c > 0 and p > 0]
    if not filtered:
        print("No data for comm_domain plot")
        return
    
    domain, comm_bytes, preproc_comm = zip(*filtered)
    
    comm_KB = [x / 1024 for x in comm_bytes]
    preproc_comm_KB = [x / 1024 for x in preproc_comm]
    
    plt.plot(domain, comm_KB, marker='o', linestyle='-', color=colors[0], label='Online Phase')
    plt.plot(domain, preproc_comm_KB, marker='s', linestyle='--', color=colors[1], label='Preprocessing Phase')
    plt.xscale('log', base=2)
    ax.set_xticks(list(domain))
    ax.set_xticklabels(list(domain))
    ax.xaxis.set_major_formatter(tick.FuncFormatter(format_number))
    ax.set_ylim(0, 135)
    ax.yaxis.set_major_formatter(tick.FuncFormatter(format_number))
    plt.xlabel('Domain Size')
    plt.ylabel('Communication Cost (KB)')
    plt.grid(True)
    plt.xticks(domain)
    plt.legend(loc='upper left')
    plt.tight_layout()
    output_file = os.path.join(output_dir, "comm_cost_domain.pdf")
    plt.savefig(output_file)
    print(f"Saved {output_file}")

# ------------------------- Scale Domain ------------------------- #
def plot_scale_domain(results, output_dir="results"):
    plt.clf()
    fig, ax = plt.subplots(figsize=(6.4, 4.8))
    plt.grid(True)
    
    domain = [128, 256, 512, 1024, 2048, 4096]
    online_time = []
    preprocessing_time = []
    
    for d in domain:
        key_online = (100, d, 'online')
        key_preprocess = (100, d, 'preprocess')
        
        if key_online in results and len(results[key_online]) > 0:
            online_time.append(statistics.mean(results[key_online]))
        else:
            online_time.append(0)
        
        if key_preprocess in results and len(results[key_preprocess]) > 0:
            preprocessing_time.append(statistics.mean(results[key_preprocess]))
        else:
            preprocessing_time.append(0)
    
    filtered = [(d, o, p) for d, o, p in zip(domain, online_time, preprocessing_time) if o > 0 and p > 0]
    if not filtered:
        print("No data for scale_domain plot")
        return
    
    domain, online_time, preprocessing_time = zip(*filtered)
    
    plt.plot(domain, online_time, marker='o', linestyle='-', color=colors[0], label='Online Phase')
    plt.plot(domain, preprocessing_time, marker='s', linestyle='--', color=colors[1], label='Preprocessing Phase')
    plt.xscale('log', base=2)
    ax.set_xticks(list(domain))
    ax.set_xticklabels(list(domain))
    ax.xaxis.set_major_formatter(tick.FuncFormatter(format_number))
    ax.set_xlim(left=110)
    ax.set_yscale('log')
    ax.set_ylim(0.6, 900)
    ax.yaxis.set_major_formatter(tick.FuncFormatter(format_number))
    plt.xlabel('Domain Size')
    plt.ylabel('Time (ms)')
    plt.xticks(domain)
    plt.grid(True)
    plt.legend(loc='upper left')
    plt.tight_layout()
    output_file = os.path.join(output_dir, "scale_domain.pdf")
    plt.savefig(output_file)
    print(f"Saved {output_file}")

def main():
    csv_file = 'results/microbenchmark_results.csv'
    output_dir = 'results'
    
    if not os.path.exists(csv_file):
        print(f"Error: {csv_file} not found. Run ./run_microbenchmarks.sh first.")
        sys.exit(1)
    
    os.makedirs(output_dir, exist_ok=True)
    
    print("Loading results...")
    results = load_results(csv_file)
    comm_results = load_comm_results(csv_file)
    
    print("Generating plots...")
    plot_comm_bidders(comm_results, output_dir)
    plot_scale_bidders(results, output_dir)
    plot_comm_domain(comm_results, output_dir)
    plot_scale_domain(results, output_dir)
    print("Done!")

if __name__ == '__main__':
    main()
