# Worst Case Ethereum Miner

A high-performance tool for mining Ethereum addresses and storage slots that create worst-case scenarios in Ethereum's Merkle Patricia Trie (MPT) structures. This tool can mine both storage slots for deep storage tries and CREATE2 addresses with auxiliary accounts for deep account tries.

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

#### Important: Deployer Address Requirements

**The `--deployer` must be a contract address, not an EOA**, since only contracts can use CREATE2. We recommend using **Nick's deterministic deployer** at `0x4e59b44847b379578588920ca78fbf26c0b4956c`, which is already deployed on Ethereum mainnet and most testnets.

```bash
# Recommended: Use Nick's deterministic deployer (already deployed on mainnet/testnets)
./target/release/worst_case_miner \
    --depth 5 \
    --num-contracts 1000 \
    --deployer 0x4e59b44847b379578588920ca78fbf26c0b4956c \
    --init-code WorstCaseERC20.sol \
    --accounts-output create2_1000_depth5.json

# Mine with pre-compiled bytecode
./target/release/worst_case_miner \
    --depth 5 \
    --num-contracts 1000 \
    --deployer 0x4e59b44847b379578588920ca78fbf26c0b4956c \
    --init-code bytecode.hex \
    --accounts-output create2_1000_depth5.json

# Auto-generate contract and mine (no init-code needed)
./target/release/worst_case_miner \
    --depth 5 \
    --num-contracts 1000 \
    --deployer 0x4e59b44847b379578588920ca78fbf26c0b4956c \
    --accounts-output create2_1000_depth5.json
```

The tool automatically compiles Solidity files with `--metadata-hash none` to ensure consistent bytecode generation.

**Note**: If you use a custom deployer contract instead of Nick's method, you must first deploy that contract and use its address. The mined addresses depend on the deployer address, so changing it will result in different CREATE2 addresses.

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
  "deployer": "0x4e59b44847b379578588920ca78fbf26c0b4956c",
  "init_code_hash": "0x90cb4a8e859fd7f27af7ea59371ffb7408be139c0e33e1436b3b2e7f3a086c4d",
  "init_code": "0x608060405234801561000f575f5ffd5b50...",
  "deploy_code": "0x608060405234801561000f575f5ffd5b50...",
  "storage_keys": [
    "0xc23303a7ec42f89ca5bf674b83a9141500ac18f0",
    "0xc5f82341cec8f50dcc8cad69423974ca8ada0f20",
    "0xed37894a02b4d0a25ea01b6e631a3915613f1bb5"
  ],
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

**Output Fields:**
- `init_code`: Full deployment bytecode (constructor + runtime) - used for CREATE2 address calculation
- `deploy_code`: Runtime bytecode only - what ends up stored on-chain after deployment
- `storage_keys`: The mined storage slot addresses that create the deep storage trie branch (only populated when using `--depth` without `--init-code`)


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