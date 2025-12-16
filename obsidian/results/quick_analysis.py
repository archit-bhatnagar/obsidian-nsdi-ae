import os
import numpy as np

print("QUICK THROUGHPUT ANALYSIS")
print("=" * 50)

for filename in sorted(os.listdir('.')):
    if filename.startswith('latencies_') and filename.endswith('_aps.txt'):
        throughput = filename.replace('latencies_', '').replace('_aps.txt', '')
        
        try:
            latencies = np.loadtxt(filename)
            if len(latencies) > 0:
                actual_throughput = len(latencies) / 60  # 60 second test
                p50 = np.percentile(latencies, 50)
                p99 = np.percentile(latencies, 99)
                print(f"{throughput:>4s} aps target: {actual_throughput:6.1f} actual, P50: {p50:6.1f}ms, P99: {p99:6.1f}ms, samples: {len(latencies)}")
        except:
            print(f"{throughput:>4s} aps target: FAILED")

