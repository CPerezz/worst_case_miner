# Worst Case Ethereum Miner

A high-performance tool for mining Ethereum addresses and storage slots that create worst-case scenarios in Ethereum's Modified Patricia Trie (MPT) structures. This tool can mine both storage slots for deep storage tries and CREATE2 addresses with auxiliary accounts for deep account tries.

![diagram.png](diagram.png)

## Features

### Storage Mining
Mines storage slots that share increasingly long prefixes, creating deep branches in ERC20 contract storage tries. This represents worst-case scenarios for storage access costs.

### Account Mining (CREATE2)
Mines CREATE2 contract addresses along with auxiliary accounts whose keccak256 hashes share prefixes, creating deep branches in the account trie. This maximizes trie traversal costs during block processing.

### Template-Based Contract Generation
Generates Solidity contracts using Jinja2 templates with hardcoded storage slots, eliminating the need for complex initcode generation.

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

### Storage Mining

Mine storage slots to create a storage branch of specified depth:

```bash
# Mine storage slots at depth 5
./target/release/worst_case_miner storage --depth 5

# Mine with specific number of threads
./target/release/worst_case_miner storage --depth 8 --threads 16

# Mine with CUDA acceleration (requires CUDA build)
./target/release/worst_case_miner storage --depth 10 --cuda
```

### CREATE2 Account Mining

Mine CREATE2 addresses with auxiliary accounts for account trie depth:

```bash
# Mine with a Solidity file (auto-compiles)
./target/release/worst_case_miner create2 \
    --depth 5 \
    --num-contracts 1000 \
    --deployer 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266 \
    --init-code WorstCaseERC20.sol \
    --accounts-output create2_1000_depth5.json

# Mine with pre-compiled bytecode
./target/release/worst_case_miner create2 \
    --depth 5 \
    --num-contracts 1000 \
    --deployer 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266 \
    --init-code bytecode.hex \
    --accounts-output create2_1000_depth5.json

# Auto-generate contract and mine (no init-code needed)
./target/release/worst_case_miner create2 \
    --depth 5 \
    --num-contracts 1000 \
    --deployer 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266 \
    --accounts-output create2_1000_depth5.json
```

The tool automatically compiles Solidity files with `--metadata-hash none` to ensure consistent bytecode generation.

### Contract Generation from Template

Generate a Solidity contract with mined storage slots:

```bash
# First mine storage slots
./target/release/worst_case_miner storage --depth 10 --output storage_depth10.json

# Contract will be generated in contracts/WorstCaseERC20.sol
```

## Output Examples

### Storage Mining Output
```json
{
  "depth": 5,
  "accounts": [
    {
      "address": "0x8179ce7275b27bf70bb579cae24c0fd7b20db7bc",
      "storage_slot": "0x704c9d618d80aa287ca6514da8e224dc98b90ef314f8d4e45c4fbf8bb4e7a94e"
    },
    {
      "address": "0x207b4fbc3a83b1eda04284bdc56d2996b54412be",
      "storage_slot": "0x7075d17623e5dfbcae458da738fcddf08a2e534ad74c72d21d07e0d81d36b42f"
    }
  ]
}
```

### CREATE2 Mining Output
```json
{
  "deployer": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
  "init_code_hash": "0x1c3374235d773b2189aed115aa13143020fcdbbe86e38f358cf3e4771b2f0244",
  "target_depth": 5,
  "num_contracts": 1000,
  "total_time": 20.328,
  "contracts": [
    {
      "salt": 0,
      "contract_address": "0x53a6a746a81797db6a0944fc32f2486c738badcb",
      "auxiliary_accounts": [
        "0x452edbff5a8cf19da307863c2e7c8b4f145ee6a1",
        "0x6ddd21179c3f2336f174783c82ef562598892c4d",
        "0xae735fd3d76b32b159afbbd6a8a2aeb8f0d1caf0",
        "0x11ceeafb90d900d1da978e230142a15ddf4b7d60",
        "0x1591037bca9d00c2824dfadf87cbd579367c0332"
      ]
    }
  ]
}
```


## Technical Details

### Account Trie Depth
The account trie uses `keccak256(address)` as keys, not the raw address. Our CREATE2 mining finds auxiliary accounts whose hashes share prefixes with the contract's hash, creating deep branches in the account trie.

### Storage Slot Calculation
Storage slots follow Solidity's mapping layout: `keccak256(address || slot)` where slot 0 is used for ERC20 balances.

### Worst-Case Trie Structure
By creating addresses/slots with shared prefixes, we force:
- Deep extension nodes before branch nodes
- Maximum trie traversal depth
- Highest computational cost per gas unit

## License

MIT