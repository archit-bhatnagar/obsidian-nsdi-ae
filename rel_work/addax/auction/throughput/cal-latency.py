import os
import re
import sys

# Pattern for individual handle times (for 2-round/interactive)
re_handle_time = re.compile(r'^(\d+) handle time:\s+(\d*\.\d*)$')
# Pattern for total time spent (for 2-round aggregate)
re_total_time = re.compile(r'^total time spent:\s+(\d*\.\d*)$')
# Pattern for TIME: total from auction-non-interactive (for non-interactive)
re_time_total = re.compile(r'^TIME:\s+total:\s+(\d*\.\d*)$')

dir_name = sys.argv[1]
time_list = []

for root, dirs, files in os.walk(dir_name, topdown=False):
    for name in files:
        file_path = os.path.join(root, name)
        # print(f"Processing: {file_path}")
        
        # Read all lines first to check format
        lines = []
        for line in open(file_path):
            lines.append(line.strip())
        
        # Check for TIME: total (non-interactive format from auction-non-interactive)
        time_total_found = []
        handle_times_found = []
        
        for line in lines:
            # First, try to match TIME: total (non-interactive)
            time_total_match = re.match(re_time_total, line)
            if time_total_match:
                time_total_found.append(float(time_total_match.group(1)))
                continue
            
            # Try to match total time spent (2-round aggregate)
            total_match = re.match(re_total_time, line)
            if total_match:
                # Skip total time spent - we use individual handle times instead
                continue
            
            # Try to match individual handle times
            handle_match = re.match(re_handle_time, line)
            if handle_match:
                handle_times_found.append(float(handle_match.group(2)))
        
        # For non-interactive: use TIME: total values if found
        if len(time_total_found) > 0:
            time_list.extend(time_total_found)
        # For 2-round: use individual handle times (skip first id 0)
        elif len(handle_times_found) > 0:
            if len(handle_times_found) == 1:
                # Only one handle time, use it
                time_list.append(handle_times_found[0])
            else:
                # Skip first entry (id 0), use rest
                time_list.extend(handle_times_found[1:])

if len(time_list) == 0:
    print("median latency: 0")
    print("99% latency: 0")
else:
    time_list.sort()
    median_idx = int(len(time_list) * 0.5) if len(time_list) > 0 else 0
    p99_idx = int(len(time_list) * 0.99) if len(time_list) > 0 else 0
    if p99_idx >= len(time_list):
        p99_idx = len(time_list) - 1
    # Convert to milliseconds for consistency with expected output
    print(f"median latency: {time_list[median_idx] * 1000:.2f}")
    print(f"99% latency: {time_list[p99_idx] * 1000:.2f}")
