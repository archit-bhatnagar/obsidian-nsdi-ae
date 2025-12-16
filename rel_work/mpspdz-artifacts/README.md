# MP-SPDZ Artifacts for Obsidian Evaluation

This directory contains the custom MPC programs and data files for evaluating the Vickrey auction implementation.

## Directory Structure

- `Programs/` - Custom MPC source files (`.mpc` programs)
- `Player-Data/` - Input data files for the MPC programs
- `Persistence/` - Pre-generated persistence files for certain protocols

## Setup Instructions

### 1. Install MP-SPDZ

Clone and build MP-SPDZ version 0.3.9:

```bash
cd rel_work/
git clone https://github.com/data61/MP-SPDZ.git mp-spdz-0.3.9
cd mp-spdz-0.3.9
git checkout v0.3.9
make -j8 tldr
```

For detailed installation instructions, see the [MP-SPDZ documentation](https://github.com/data61/MP-SPDZ).

### 2. Copy Artifacts

Use the provided setup script to copy the artifacts into your MP-SPDZ installation:

```bash
cd rel_work/mpspdz-artifacts/
./setup_mpspdz.sh
```

Or manually copy the files:

```bash
# From the mpspdz-artifacts directory
cp Programs/* ../mp-spdz-0.3.9/Programs/Source/
cp Player-Data/* ../mp-spdz-0.3.9/Player-Data/
cp Persistence/* ../mp-spdz-0.3.9/Persistence/
```

### 3. Compile and Run

Compile the Vickrey auction program:

```bash
cd ../mp-spdz-0.3.9/
./compile.py vickrey
```

Run with the MASCOT protocol:

```bash
Scripts/mascot.sh -v vickrey
```

To clear secret share files between runs:

```bash
./clear_persist.sh
```

## Files Included

### Programs
- `vickrey.mpc` - Vickrey auction MPC program

### Player-Data
- `bids_party0.txt` - Bid data for party 0
- `bids_party1.txt` - Bid data for party 1

### Persistence
- `Transactions-P0.data` - Persistence data for party 0
- `Transactions-P1.data` - Persistence data for party 1

### Scripts
- `clear_persist.sh` - Script to clear secret share files between runs

## Notes

- The persistence files are optional and only needed for certain protocols
- Evaluators should install MP-SPDZ themselves to ensure a clean environment
- All source files in this directory are the custom artifacts for the evaluation

