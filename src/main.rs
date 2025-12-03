use clap::Parser;
use tiny_keccak::{Keccak, Hasher};
use std::time::Instant;
use hex;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::thread;
use log::{info, debug};

#[cfg(feature = "cuda")]
mod cuda_miner;

/// Standard ERC20 balance mapping storage slot
/// In OpenZeppelin's ERC20 implementation, _balances is the first state variable (slot 0)
const ERC20_BALANCES_SLOT: u64 = 0;

/// A mining program to create deep branches in ERC20 contract storage
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target depth for the storage branch
    #[arg(short, long)]
    depth: usize,

    /// Number of threads to use for mining (default: number of CPU cores)
    #[arg(short, long, default_value_t = num_cpus::get())]
    threads: usize,

    /// Use CUDA acceleration if available
    #[arg(long)]
    cuda: bool,
}

#[derive(Clone, Debug)]
struct StorageSlot {
    address: [u8; 20],
    storage_key: [u8; 32],
    depth: usize,
}

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    info!("Starting mining for depth: {}", args.depth);

    #[cfg(feature = "cuda")]
    {
        if args.cuda && cuda_miner::cuda_available() {
            info!("Using CUDA acceleration");
        } else if args.cuda {
            info!("CUDA requested but not available, falling back to CPU");
            info!("Using {} CPU threads", args.threads);
        } else {
            info!("Using {} CPU threads", args.threads);
        }
    }

    #[cfg(not(feature = "cuda"))]
    {
        if args.cuda {
            info!("CUDA support not compiled. Rebuild with --features cuda");
        }
        info!("Using {} CPU threads", args.threads);
    }

    let start_time = Instant::now();

    // Mine for the deep branch
    let branch = mine_deep_branch(args.depth, args.threads, args.cuda);

    let elapsed = start_time.elapsed();

    // Output results
    print_results(&branch, elapsed.as_secs_f64());

    // Generate and output initcode
    let _initcode = generate_initcode(&branch);
}

/// Calculate the storage slot for a given address in the balances mapping
fn calculate_storage_slot(address: &[u8; 20], base_slot: u64) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut storage_key = [0u8; 32];

    // For mappings in Solidity: keccak256(key || slot)
    // Key is the address (padded to 32 bytes)
    let mut padded_address = [0u8; 32];
    padded_address[12..32].copy_from_slice(address);

    // Slot index (padded to 32 bytes)
    let mut slot_bytes = [0u8; 32];
    slot_bytes[24..32].copy_from_slice(&base_slot.to_be_bytes());

    // Hash the concatenated data
    hasher.update(&padded_address);
    hasher.update(&slot_bytes);
    hasher.finalize(&mut storage_key);

    storage_key
}

/// Mine for a deep branch by finding addresses sequentially, one depth at a time
fn mine_deep_branch(target_depth: usize, num_threads: usize, use_cuda: bool) -> Vec<StorageSlot> {
    let mut branch = Vec::new();

    info!("Starting sequential mining for {} levels", target_depth);

    // For each depth level, find an address that creates the right prefix collision
    for current_depth in 0..target_depth {
        let level_start = Instant::now();

        // Each level should share an increasing number of nibbles:
        // Level 1: any address (0 shared nibbles required)
        // Level 2: 1 shared nibble with level 1
        // Level 3: 2 shared nibbles with levels 1 & 2
        // Level N: N-1 shared nibbles with all previous levels
        let required_prefix_nibbles = current_depth;

        info!("Mining level {}/{} (requires {} matching nibbles)",
              current_depth + 1, target_depth, required_prefix_nibbles);

        // Mine for an address at this depth level
        let address = if current_depth == 0 {
            // First address can be anything - just generate a random one
            let mut rng = rand::thread_rng();
            let mut addr = [0u8; 20];
            rng.fill(&mut addr);
            addr
        } else {
            // Need to find an address that shares the required prefix with the PREVIOUS level
            // (not all previous addresses, just the immediately preceding one)
            let previous_slot: &StorageSlot = &branch[branch.len() - 1];
            // Only use CUDA for depth 8+ where the computational cost justifies the overhead
            let use_cuda_for_level = use_cuda && current_depth >= 8;
            match mine_address_for_prefix(&previous_slot.storage_key, required_prefix_nibbles, num_threads, use_cuda_for_level) {
                Some(addr) => addr,
                None => {
                    info!("Failed to find address for level {} - stopping", current_depth + 1);
                    break;
                }
            }
        };

        let storage_key = calculate_storage_slot(&address, ERC20_BALANCES_SLOT);

        branch.push(StorageSlot {
            address,
            storage_key,
            depth: current_depth,
        });

        let level_time = level_start.elapsed();
        info!("Level {} found in {:.2} seconds - Address: 0x{}, Storage: 0x{}...",
              current_depth + 1,
              level_time.as_secs_f64(),
              hex::encode(&address[..4]),
              hex::encode(&storage_key[..4]));
    }

    branch
}

/// Mine for a single address that shares a prefix with the target storage key
fn mine_address_for_prefix(
    target_storage_key: &[u8; 32],
    required_prefix_nibbles: usize,
    num_threads: usize,
    #[allow(unused_variables)]
    use_cuda: bool
) -> Option<[u8; 20]> {
    #[cfg(feature = "cuda")]
    {
        if use_cuda && cuda_miner::cuda_available() {
            info!("Using CUDA acceleration for level with {} required nibbles", required_prefix_nibbles);
            // Try CUDA mining first
            if let Some((address, _storage_key)) = cuda_miner::mine_with_cuda(
                target_storage_key,
                required_prefix_nibbles,
                ERC20_BALANCES_SLOT
            ) {
                return Some(address);
            }
            info!("CUDA mining failed, falling back to CPU");
        }
    }
    let result = Arc::new(Mutex::new(None));
    let found = Arc::new(Mutex::new(false));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let result_clone = Arc::clone(&result);
            let found_clone = Arc::clone(&found);
            let target = target_storage_key.clone();

            thread::spawn(move || {
                mine_worker_for_prefix(
                    thread_id,
                    &target,
                    required_prefix_nibbles,
                    result_clone,
                    found_clone
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    result.lock().unwrap().clone()
}

fn mine_worker_for_prefix(
    thread_id: usize,
    target_prefix: &[u8; 32],
    required_nibbles: usize,
    result: Arc<Mutex<Option<[u8; 20]>>>,
    found: Arc<Mutex<bool>>,
) {
    let mut rng = rand::thread_rng();
    let mut attempts = 0u64;

    // Pre-compute the slot bytes since they don't change
    let mut slot_bytes = [0u8; 32];
    slot_bytes[24..32].copy_from_slice(&ERC20_BALANCES_SLOT.to_be_bytes());

    // Batch size for checking - check found flag less often
    const BATCH_SIZE: u64 = 1000;

    loop {
        // Check if another thread found a result (but only every BATCH_SIZE attempts)
        if attempts % BATCH_SIZE == 0 && *found.lock().unwrap() {
            break;
        }

        attempts += 1;
        if attempts % 1000000 == 0 {
            debug!("Thread {}: {} million attempts", thread_id, attempts / 1000000);
        }

        // Generate a random address
        let mut address = [0u8; 20];
        rng.fill(&mut address);

        // Calculate storage key inline for better performance
        use tiny_keccak::{Keccak, Hasher};
        let mut hasher = Keccak::v256();
        let mut storage_key = [0u8; 32];

        // Prepare padded address
        let mut padded_address = [0u8; 32];
        padded_address[12..32].copy_from_slice(&address);

        // Hash in one go
        hasher.update(&padded_address);
        hasher.update(&slot_bytes);
        hasher.finalize(&mut storage_key);

        // Check if it matches the required prefix
        if has_nibble_prefix(&storage_key, target_prefix, required_nibbles) {
            let mut found_lock = found.lock().unwrap();
            if !*found_lock {
                *found_lock = true;
                let mut result_lock = result.lock().unwrap();
                *result_lock = Some(address);
                info!("Thread {} found matching address after {} attempts", thread_id, attempts);
            }
            break;
        }
    }
}

/// Check if two storage keys share a prefix of the specified number of nibbles
fn has_nibble_prefix(a: &[u8; 32], b: &[u8; 32], nibbles: usize) -> bool {
    if nibbles == 0 {
        return true;
    }

    let full_bytes = nibbles / 2;
    let has_half_byte = nibbles % 2 == 1;

    // Check full bytes
    for i in 0..full_bytes {
        if a[i] != b[i] {
            return false;
        }
    }

    // Check the half byte (single nibble) if needed
    if has_half_byte && full_bytes < 32 {
        let mask = 0xF0; // Check only the high nibble
        if (a[full_bytes] & mask) != (b[full_bytes] & mask) {
            return false;
        }
    }

    true
}

fn print_results(branch: &[StorageSlot], elapsed_seconds: f64) {
    info!("");
    info!("╔════════════════════════════════════════════════════════════════════════╗");
    info!("║                          MINING RESULTS                                ║");
    info!("╚════════════════════════════════════════════════════════════════════════╝");
    info!("");
    info!("Total depth achieved: {}", branch.len());
    info!("Total time taken: {:.2} seconds", elapsed_seconds);
    info!("ERC20 balance mapping slot: {}", ERC20_BALANCES_SLOT);
    info!("");
    info!("═══ Branch Structure (Sequential Addresses) ═══");
    info!("");

    // Show the common prefix that all addresses share
    if branch.len() > 1 {
        let common_nibbles = branch.len() - 1;
        let common_prefix = get_common_prefix(&branch);
        info!("Common prefix ({} nibbles): 0x{}", common_nibbles, common_prefix);
        info!("");
    }

    // Print each address in the branch
    for (i, slot) in branch.iter().enumerate() {
        info!("Level {} (Depth {}):", i + 1, slot.depth);
        info!("  Address:     0x{}", hex::encode(slot.address));
        info!("  Storage Key: 0x{}", hex::encode(slot.storage_key));

        if i > 0 {
            // Show how many nibbles this shares with the previous level
            let shared = count_shared_nibbles(&branch[i-1].storage_key, &slot.storage_key);
            info!("  Shares {} nibbles with previous level", shared);
        }
        info!("");
    }

    info!("═══ Statistics ═══");
    info!("Total addresses mined: {}", branch.len());
    info!("Average time per level: {:.2} seconds", elapsed_seconds / branch.len() as f64);

    // Estimate the number of hashes computed
    let total_attempts_estimate: u64 = branch.iter().enumerate()
        .map(|(i, _)| if i == 0 { 1 } else { 16_u64.pow(i as u32) })
        .sum();
    info!("Estimated total hash computations: ~{}", format_number(total_attempts_estimate));
}

/// Get the common prefix shared by all addresses in the branch
fn get_common_prefix(branch: &[StorageSlot]) -> String {
    if branch.is_empty() {
        return String::new();
    }

    let first_key = &branch[0].storage_key;
    let min_shared = branch.len() - 1;

    // Convert to hex and take the appropriate number of nibbles
    let hex_str = hex::encode(first_key);
    hex_str.chars().take(min_shared).collect()
}

/// Count how many nibbles two storage keys share
fn count_shared_nibbles(a: &[u8; 32], b: &[u8; 32]) -> usize {
    let hex_a = hex::encode(a);
    let hex_b = hex::encode(b);

    hex_a.chars().zip(hex_b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .count()
}

/// Format large numbers with commas for readability
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Generate EVM initcode that deploys a contract with all mined addresses pre-loaded
fn generate_initcode(branch: &[StorageSlot]) -> Vec<u8> {
    info!("");
    info!("╔════════════════════════════════════════════════════════════════════════╗");
    info!("║                         EVM INITCODE GENERATION                        ║");
    info!("╚════════════════════════════════════════════════════════════════════════╝");
    info!("");

    let mut bytecode = Vec::new();

    // For each mined address, generate SSTORE operations
    // Each SSTORE needs:
    // 1. PUSH32 <value> (1 wei)
    // 2. PUSH32 <storage_key>
    // 3. SSTORE

    for slot in branch {
        // PUSH1 0x01 (value = 1 wei)
        bytecode.push(0x60); // PUSH1
        bytecode.push(0x01); // value: 1

        // PUSH32 <storage_key>
        bytecode.push(0x7f); // PUSH32
        bytecode.extend_from_slice(&slot.storage_key);

        // SSTORE
        bytecode.push(0x55); // SSTORE opcode
    }

    // After all SSTOREs, deploy minimal ERC20 runtime code
    // For simplicity, we'll deploy a minimal contract that just stores the data

    // Simple runtime code that just has a fallback function
    let runtime_code = vec![
        0x00, // STOP (minimal runtime - just stores the data)
    ];

    // Calculate runtime code size
    let runtime_size = runtime_code.len();

    // CODECOPY the runtime code to memory
    // PUSH1 <runtime_size>
    bytecode.push(0x60);
    bytecode.push(runtime_size as u8);

    // PUSH1 0x00 (memory destination)
    bytecode.push(0x60);
    bytecode.push(0x00);

    // Calculate position of runtime code in bytecode
    let runtime_offset = bytecode.len() + 4; // +4 for CODECOPY and RETURN opcodes

    // PUSH2 <runtime_offset>
    if runtime_offset < 256 {
        bytecode.push(0x60); // PUSH1
        bytecode.push(runtime_offset as u8);
    } else {
        bytecode.push(0x61); // PUSH2
        bytecode.push((runtime_offset >> 8) as u8);
        bytecode.push((runtime_offset & 0xFF) as u8);
    }

    // CODECOPY
    bytecode.push(0x39);

    // RETURN the runtime code
    // PUSH1 <runtime_size>
    bytecode.push(0x60);
    bytecode.push(runtime_size as u8);

    // PUSH1 0x00 (memory offset)
    bytecode.push(0x60);
    bytecode.push(0x00);

    // RETURN
    bytecode.push(0xf3);

    // Append the runtime code
    bytecode.extend_from_slice(&runtime_code);

    // Output the initcode
    info!("Generated initcode ({} bytes):", bytecode.len());
    info!("");

    // Output as hex string
    let hex_string = hex::encode(&bytecode);

    // Break into chunks for readability (with log formatting)
    for (i, chunk) in hex_string.as_bytes().chunks(64).enumerate() {
        if i == 0 {
            info!("0x{}", std::str::from_utf8(chunk).unwrap());
        } else {
            info!("  {}", std::str::from_utf8(chunk).unwrap());
        }
    }

    // Also output the complete initcode in a copy-friendly format
    info!("");
    info!("═══ Complete Initcode (copy-friendly) ═══");
    println!("0x{}", hex_string);

    info!("");
    info!("═══ Initcode Breakdown ═══");
    info!("SSTOREs: {} operations ({} bytes)", branch.len(), branch.len() * 35);
    info!("Deployment overhead: {} bytes", bytecode.len() - (branch.len() * 35));
    info!("Total initcode size: {} bytes", bytecode.len());

    // Calculate deployment gas estimate
    let gas_estimate = estimate_deployment_gas(branch.len(), bytecode.len());
    info!("Estimated deployment gas: ~{}", format_number(gas_estimate));

    info!("");
    info!("═══ Storage Slots Written ═══");
    for (i, slot) in branch.iter().enumerate() {
        info!("Slot {}: 0x{}", i + 1, hex::encode(&slot.storage_key));
    }

    // Return the bytecode
    bytecode
}

/// Estimate gas cost for deploying the contract
fn estimate_deployment_gas(num_sstores: usize, bytecode_size: usize) -> u64 {
    let base_creation = 32000; // Base contract creation cost
    let per_byte = 200; // Gas per byte of initcode (approximate)
    let sstore_cost = 20000; // Cold SSTORE cost (first write to slot)

    base_creation + (bytecode_size as u64 * per_byte) + (num_sstores as u64 * sstore_cost)
}