# Worst Case Storage Miner

A high-performance tool for mining Ethereum storage slots that create worst-case scenarios in the ERC20 balance mapping storage trie. This tool finds addresses whose storage keys share increasingly long prefixes, forcing deep branches in the Modified Patricia Trie (MPT) structure.

![diagram.png](diagram.png)

## What it does

This miner generates addresses that, when used in an ERC20 contract's balance mapping, create storage keys with cascading common prefixes. This results in deep branches in the storage trie, which represents the worst-case scenario for storage access costs.

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/worst_case_miner
cd worst_case_miner

# Build with optimizations (CPU only)
cargo build --release

# Optional: Build with CUDA support (requires NVIDIA GPU and CUDA toolkit)
cargo build --release --features cuda
```

## Usage

### Basic Usage

Mine addresses to create a storage branch of depth 5:

```bash
./target/release/worst_case_miner --depth 5
```

### Advanced Options

```bash
# Specify number of CPU threads
./target/release/worst_case_miner --depth 8 --threads 16

# Use CUDA acceleration (requires CUDA build, auto-enables for depth 8+)
./target/release/worst_case_miner --depth 12 --cuda

# Enable debug logging to see thread progress
RUST_LOG=debug ./target/release/worst_case_miner --depth 6
```

## Example Output

```
[INFO] Starting mining for depth: 5
[INFO] Using 14 CPU threads
[INFO] Starting sequential mining for 5 levels
[INFO] Mining level 1/5 (requires 0 matching nibbles)
[INFO] Level 1 found in 0.00 seconds - Address: 0x8179ce72, Storage: 0x704c9d61...
[INFO] Mining level 2/5 (requires 1 matching nibbles)
[INFO] Level 2 found in 0.00 seconds - Address: 0x207b4fbc, Storage: 0x7075d176...
[INFO] Mining level 3/5 (requires 2 matching nibbles)
[INFO] Level 3 found in 0.00 seconds - Address: 0xc0941606, Storage: 0x70de573f...
...

═══ Branch Structure (Sequential Addresses) ═══

Common prefix (4 nibbles): 0x704c

Level 1 (Depth 0):
  Address:     0x8179ce7275b27bf70bb579cae24c0fd7b20db7bc
  Storage Key: 0x704c9d618d80aa287ca6514da8e224dc98b90ef314f8d4e45c4fbf8bb4e7a94e

Level 2 (Depth 1):
  Address:     0x207b4fbc3a83b1eda04284bdc56d2996b54412be
  Storage Key: 0x7075d17623e5dfbcae458da738fcddf08a2e534ad74c72d21d07e0d81d36b42f
  Shares 1 nibbles with previous level

...

═══ EVM INITCODE GENERATION ═══

Generated initcode (193 bytes):
0x60017f704c9d618d80aa287ca6514da8e224dc98b90ef314f8d4e45c4fbf8bb4...
```

## Output Explanation

The tool outputs:
1. **Mined Addresses**: Ethereum addresses that create the desired storage pattern
2. **Storage Keys**: The keccak256 hashes used as storage slots in the ERC20 balance mapping
3. **Shared Prefixes**: Shows how many nibbles each level shares with the previous one
4. **EVM Initcode**: Ready-to-deploy bytecode that stores all mined values in a contract

## Performance

On Apple M4 Pro (CPU only):
- **Hash rate**: ~68.5 million hashes/second
- **Depth 5**: Instant
- **Depth 8**: ~4 seconds
- **Depth 9**: ~1 minute
- **Depth 10**: ~15 minutes

## Deployment Example

Deploy the generated contract to Anvil (local Ethereum node):

```bash
# Start Anvil
anvil

# Deploy the generated initcode (copy the hex output from the miner)
cast send --rpc-url http://localhost:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
  --create "0x[PASTE_INITCODE_HERE]"

# Verify storage was written
cast storage --rpc-url http://localhost:8545 [CONTRACT_ADDRESS] [STORAGE_KEY]
```


## Technical Details

- Storage slot calculation follows Solidity's mapping storage layout: `keccak256(address || slot)`
- Uses OpenZeppelin's standard ERC20 layout (balances at slot 0)
- Creates worst-case MPT structure by forcing deep extension nodes before branch nodes
- Mining difficulty increases exponentially with depth (16^n possibilities for n nibbles)

## License

MIT