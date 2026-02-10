#!/bin/bash

# Setup script for copying MP-SPDZ artifacts
# This script copies custom MPC programs and data into an MP-SPDZ installation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MPSPDZ_DIR="${SCRIPT_DIR}/../mp-spdz-0.3.9"

echo "MP-SPDZ Artifacts Setup"
echo "======================="
echo ""

# Check if MP-SPDZ is installed
if [ ! -d "$MPSPDZ_DIR" ]; then
    echo "Error: MP-SPDZ directory not found at: $MPSPDZ_DIR"
    echo ""
    echo "Please install MP-SPDZ first:"
    echo "  cd $(dirname "$MPSPDZ_DIR")"
    echo "  git clone https://github.com/data61/MP-SPDZ.git mp-spdz-0.3.9"
    echo "  cd mp-spdz-0.3.9"
    echo "  git checkout v0.3.9"
    echo "  make -j8 tldr"
    echo ""
    exit 1
fi

echo "Found MP-SPDZ installation at: $MPSPDZ_DIR"
echo ""

# Copy Programs
echo "Copying MPC programs..."
cp -v "${SCRIPT_DIR}/Programs"/* "${MPSPDZ_DIR}/Programs/Source/"

# Copy Player Data
echo ""
echo "Copying player data..."
cp -v "${SCRIPT_DIR}/Player-Data"/* "${MPSPDZ_DIR}/Player-Data/"

# Copy scripts
echo ""
echo "Copying utility scripts..."
cp -v "${SCRIPT_DIR}/clear_persist.sh" "${MPSPDZ_DIR}/"
cp -v "${SCRIPT_DIR}/generate_inputs.py" "${MPSPDZ_DIR}/"
cp -v "${SCRIPT_DIR}/run_network_benchmark.sh" "${MPSPDZ_DIR}/"
chmod +x "${MPSPDZ_DIR}/clear_persist.sh"
chmod +x "${MPSPDZ_DIR}/generate_inputs.py"
chmod +x "${MPSPDZ_DIR}/run_network_benchmark.sh"

echo ""
echo "✓ Setup complete!"
echo ""
echo "Next steps:"
echo "  cd $MPSPDZ_DIR"
echo "  ./compile.py vickrey"
echo "  Scripts/mascot.sh -v vickrey"
echo ""
echo "To run network benchmarks with RTT emulation:"
echo "  sudo ./run_network_benchmark.sh <num_bidders> <rtt_ms> <num_runs>"
echo "  Example: sudo ./run_network_benchmark.sh 104 20 3"
echo ""
echo "To clear secret shares between runs:"
echo "  ./clear_persist.sh"
echo ""

