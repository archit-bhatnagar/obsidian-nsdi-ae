#!/usr/bin/env python3
"""
Generate Input-P0-0 and Input-P1-0 files for MP-SPDZ Vickrey auction benchmarks.
Each file contains 'domain_size' number of random bid values (0-100).
"""

import random
import sys

def generate_inputs(domain_size, output_dir="Player-Data"):
    """Generate Input-P0-0 and Input-P1-0 files with domain_size bids each."""
    random.seed(42)  # Deterministic for reproducibility
    
    # Generate bids for party 0
    with open(f"{output_dir}/Input-P0-0", "w") as f:
        for _ in range(domain_size):
            bid = random.randint(0, 100)
            f.write(f"{bid}\n")
    
    # Generate bids for party 1
    with open(f"{output_dir}/Input-P1-0", "w") as f:
        for _ in range(domain_size):
            bid = random.randint(0, 100)
            f.write(f"{bid}\n")
    
    print(f"Generated Input-P0-0 and Input-P1-0 with {domain_size} bids each")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 generate_inputs.py <domain_size> [output_dir]")
        sys.exit(1)
    
    domain_size = int(sys.argv[1])
    output_dir = sys.argv[2] if len(sys.argv) > 2 else "Player-Data"
    
    generate_inputs(domain_size, output_dir)

