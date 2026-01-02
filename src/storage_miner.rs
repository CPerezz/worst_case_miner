//! # Storage Miner Module
//!
//! This module provides functionality for mining Ethereum storage slots that create worst-case
//! scenarios in ERC20 contract storage tries. It finds addresses whose storage keys share
//! increasingly long prefixes, forcing deep branches in the Modified Patricia Trie structure.
//!
//! ## Key Functions
//! - `mine_deep_branch`: Mines a sequence of addresses creating a deep storage trie branch
//! - `calculate_storage_slot`: Computes the storage slot for an address in an ERC20 balance mapping
//! - `generate_contract`: Creates a Solidity contract with the mined storage slots

use askama::Template;
use log::{debug, info};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tiny_keccak::{Hasher, Keccak};

#[cfg(feature = "cuda")]
use crate::cuda_miner;

/// Template for generating Solidity contract
#[derive(Template)]
#[template(path = "WorstCaseERC20.sol.j2")]
pub struct ContractTemplate {
    addresses: Vec<String>,
}

/// Standard ERC20 balance mapping storage slot
/// In OpenZeppelin's ERC20 implementation, _balances is the first state variable (slot 0)
pub const ERC20_BALANCES_SLOT: u64 = 0;

#[derive(Clone, Debug)]
pub struct StorageSlot {
    pub address: [u8; 20],
    pub storage_key: [u8; 32],
    pub depth: usize,
    pub time_taken: f64, // Time taken to mine this level in seconds
}

/// Calculate the storage slot for a given address in the balances mapping
pub fn calculate_storage_slot(address: &[u8; 20], base_slot: u64) -> [u8; 32] {
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
pub fn mine_deep_branch(
    target_depth: usize,
    num_threads: usize,
    use_cuda: bool,
) -> Vec<StorageSlot> {
    let mut branch = Vec::new();

    info!("Starting sequential mining for {target_depth} levels");

    // For each depth level, find an address that creates the right prefix collision
    for current_depth in 0..target_depth {
        let level_start = Instant::now();

        // Each level should share an increasing number of nibbles:
        // Level 1: any address (0 shared nibbles required)
        // Level 2: 1 shared nibble with level 1
        // Level 3: 2 shared nibbles with levels 1 & 2
        // Level N: N-1 shared nibbles with all previous levels
        let required_prefix_nibbles = current_depth;

        info!(
            "Mining level {}/{} (requires {} matching nibbles)",
            current_depth + 1,
            target_depth,
            required_prefix_nibbles
        );

        // Mine for an address at this depth level
        let address = if current_depth == 0 {
            // First address can be anything - just generate a random one
            let mut addr = [0u8; 20];
            fastrand::fill(&mut addr);
            addr
        } else {
            // Need to find an address that shares the required prefix with the PREVIOUS level
            // (not all previous addresses, just the immediately preceding one)
            let previous_slot: &StorageSlot = &branch[branch.len() - 1];
            // Only use CUDA for depth 8+ where the computational cost justifies the overhead
            let use_cuda_for_level = use_cuda && current_depth >= 8;
            match mine_address_for_prefix(
                &previous_slot.storage_key,
                required_prefix_nibbles,
                num_threads,
                use_cuda_for_level,
            ) {
                Some(addr) => addr,
                None => {
                    info!(
                        "Failed to find address for level {} - stopping",
                        current_depth + 1
                    );
                    break;
                }
            }
        };

        let storage_key = calculate_storage_slot(&address, ERC20_BALANCES_SLOT);

        let level_time = level_start.elapsed();

        branch.push(StorageSlot {
            address,
            storage_key,
            depth: current_depth,
            time_taken: level_time.as_secs_f64(),
        });

        info!(
            "Level {} found in {:.2} seconds - Address: 0x{}, Storage: 0x{}...",
            current_depth + 1,
            level_time.as_secs_f64(),
            hex::encode(&address[..4]),
            hex::encode(&storage_key[..4])
        );
    }

    branch
}

/// Mine for a single address that shares a prefix with the target storage key
fn mine_address_for_prefix(
    target_storage_key: &[u8; 32],
    required_prefix_nibbles: usize,
    num_threads: usize,
    #[allow(unused_variables)] use_cuda: bool,
) -> Option<[u8; 20]> {
    #[cfg(feature = "cuda")]
    {
        if use_cuda && cuda_miner::cuda_available() {
            info!(
                "Using CUDA acceleration for level with {} required nibbles",
                required_prefix_nibbles
            );
            // Try CUDA mining first
            if let Some((address, _storage_key)) = cuda_miner::mine_with_cuda(
                target_storage_key,
                required_prefix_nibbles,
                ERC20_BALANCES_SLOT,
            ) {
                return Some(address);
            }
            info!("CUDA mining failed, falling back to CPU");
        }
    }
    let result = Arc::new(Mutex::new(None));
    let found = Arc::new(AtomicBool::new(false));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let result_clone = Arc::clone(&result);
            let found_clone = Arc::clone(&found);
            let target = *target_storage_key;

            thread::spawn(move || {
                mine_worker_for_prefix(
                    thread_id,
                    &target,
                    required_prefix_nibbles,
                    result_clone,
                    found_clone,
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    *result.lock().unwrap()
}

fn mine_worker_for_prefix(
    thread_id: usize,
    target_prefix: &[u8; 32],
    required_nibbles: usize,
    result: Arc<Mutex<Option<[u8; 20]>>>,
    found: Arc<AtomicBool>,
) {
    // Use fastrand for faster non-crypto PRNG
    let mut rng = fastrand::Rng::new();
    let mut attempts = 0u64;

    // Pre-compute the slot bytes since they don't change
    let mut slot_bytes = [0u8; 32];
    slot_bytes[24..32].copy_from_slice(&ERC20_BALANCES_SLOT.to_be_bytes());

    // Batch size for checking - check found flag less often
    const BATCH_SIZE: u64 = 1000;

    loop {
        // Check if another thread found a result (but only every BATCH_SIZE attempts)
        // Using Relaxed ordering - sufficient for a stop flag, no synchronization needed
        if attempts % BATCH_SIZE == 0 && found.load(Ordering::Relaxed) {
            break;
        }

        attempts += 1;
        if attempts % 1000000 == 0 {
            debug!(
                "Thread {}: {} million attempts",
                thread_id,
                attempts / 1000000
            );
        }

        // Generate a random address using fastrand
        let mut address = [0u8; 20];
        rng.fill(&mut address);

        // Calculate storage key inline for better performance
        use tiny_keccak::{Hasher, Keccak};
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
            // Use compare_exchange for atomic flag setting
            if found
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                let mut result_lock = result.lock().unwrap();
                *result_lock = Some(address);
                info!("Thread {thread_id} found matching address after {attempts} attempts");
            }
            break;
        }
    }
}

/// Check if two storage keys share a prefix of the specified number of nibbles
pub fn has_nibble_prefix(a: &[u8; 32], b: &[u8; 32], nibbles: usize) -> bool {
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

pub fn print_results(branch: &[StorageSlot], elapsed_seconds: f64) {
    info!("");
    info!("╔════════════════════════════════════════════════════════════════════════╗");
    info!("║                          MINING RESULTS                                ║");
    info!("╚════════════════════════════════════════════════════════════════════════╝");
    info!("");
    info!("Total depth achieved: {}", branch.len());
    info!("Total time taken: {elapsed_seconds:.2} seconds");
    info!("ERC20 balance mapping slot: {ERC20_BALANCES_SLOT}");
    info!("");
    info!("═══ Branch Structure (Sequential Addresses) ═══");
    info!("");

    // Show the common prefix that all addresses share
    if branch.len() > 1 {
        let common_nibbles = branch.len() - 1;
        let common_prefix = get_common_prefix(branch);
        info!("Common prefix ({common_nibbles} nibbles): 0x{common_prefix}");
        info!("");
    }

    // Print each address in the branch
    for (i, slot) in branch.iter().enumerate() {
        info!("Level {} (Depth {}):", i + 1, slot.depth);
        info!("  Address:     0x{}", hex::encode(slot.address));
        info!("  Storage Key: 0x{}", hex::encode(slot.storage_key));

        if i > 0 {
            // Show how many nibbles this shares with the previous level
            let shared = count_shared_nibbles(&branch[i - 1].storage_key, &slot.storage_key);
            info!("  Shares {shared} nibbles with previous level");
        }
        info!("");
    }

    info!("═══ Statistics ═══");
    info!("Total addresses mined: {}", branch.len());
    info!("");
    info!("Time per depth level:");
    for (i, slot) in branch.iter().enumerate() {
        info!(
            "  Level {} (depth {}): {:.2} seconds",
            i + 1,
            slot.depth,
            slot.time_taken
        );
    }
    info!("");
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

    hex_a
        .chars()
        .zip(hex_b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .count()
}

/// Generate and compile the Solidity contract with hardcoded storage keys
pub fn generate_contract(branch: &[StorageSlot]) {
    info!("");
    info!("╔════════════════════════════════════════════════════════════════════════╗");
    info!("║                     CONTRACT GENERATION & COMPILATION                  ║");
    info!("╚════════════════════════════════════════════════════════════════════════╝");
    info!("");

    // Step 1: Generate the contract using Askama template
    let addresses: Vec<String> = branch
        .iter()
        .map(|slot| hex::encode(slot.address))
        .collect();

    let template = ContractTemplate {
        addresses: addresses.clone(),
    };

    let contract_source = match template.render() {
        Ok(source) => source,
        Err(e) => {
            log::error!("Failed to render contract template: {e}");
            return;
        }
    };

    // Ensure contracts directory exists
    if let Err(e) = fs::create_dir_all("contracts") {
        log::error!("Failed to create contracts directory: {e}");
        return;
    }

    // Save the generated contract
    let contract_path = "contracts/WorstCaseERC20.sol";
    if let Err(e) = fs::write(contract_path, &contract_source) {
        log::error!("Failed to write contract: {e}");
        return;
    }
    info!("Generated contract saved to: {contract_path}");
}
