//! # Account Miner Module
//!
//! This module provides functionality for mining CREATE2 contract addresses and auxiliary accounts
//! that create deep branches in Ethereum's account trie. It generates accounts whose keccak256
//! hashes share prefixes, forcing deep trie traversals during block processing.
//!
//! ## Key Functions
//! - `mine_create2_accounts`: Main entry point for mining CREATE2 contracts with auxiliary accounts
//! - `calculate_create2_address`: Computes deterministic CREATE2 addresses
//! - `mine_auxiliaries_for_contract`: Mines accounts whose hashes share prefixes with a contract

use log::{debug, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tiny_keccak::{Hasher, Keccak};

/// Result structure for CREATE2-based mining
#[derive(Serialize, Deserialize)]
pub struct Create2MiningResult {
    pub deployer: String,
    pub init_code_hash: String,
    pub init_code: String,
    pub deploy_code: String,
    pub storage_keys: Vec<String>,
    pub target_depth: usize,
    pub num_contracts: usize,
    pub total_time: f64,
    pub contracts: Vec<ContractWithAuxiliaries>,
}

/// Contract with its auxiliary accounts
#[derive(Serialize, Deserialize)]
pub struct ContractWithAuxiliaries {
    pub salt: u32,
    pub contract_address: String,
    pub auxiliary_accounts: Vec<String>,
}

/// Main entry point for CREATE2-based account mining
pub fn mine_create2_accounts(
    deployer: [u8; 20],
    num_contracts: usize,
    target_depth: usize,
    num_threads: usize,
    init_code: &[u8],
    deploy_code: &[u8],
    storage_keys: &[[u8; 20]],
    output_path: &str,
) {
    info!("");
    info!("╔════════════════════════════════════════════════════════════════════════╗");
    info!("║                      CREATE2 ACCOUNT MINING MODE                       ║");
    info!("╚════════════════════════════════════════════════════════════════════════╝");
    info!("");
    info!("Deployer: 0x{}", hex::encode(deployer));
    info!("Contracts to deploy: {num_contracts}");
    info!("Target trie depth: {target_depth}");
    info!("Mining threads: {num_threads}");
    info!("");

    let total_start = Instant::now();

    // Calculate init code hash
    let init_code_hash = keccak256(init_code);
    info!("Init code hash: 0x{}", hex::encode(init_code_hash));

    let mut contracts = Vec::new();

    // Process each contract
    for contract_idx in 0..num_contracts {
        let salt = contract_idx as u32;

        // Calculate CREATE2 address
        let contract_address = calculate_create2_address(&deployer, salt, &init_code_hash);

        info!(
            "Contract {}/{} - Address: 0x{}...",
            contract_idx + 1,
            num_contracts,
            hex::encode(&contract_address[..4])
        );

        // Mine auxiliary accounts for this contract
        let auxiliaries =
            mine_auxiliaries_for_contract(&contract_address, target_depth, num_threads);

        contracts.push(ContractWithAuxiliaries {
            salt,
            contract_address: format!("0x{}", hex::encode(contract_address)),
            auxiliary_accounts: auxiliaries
                .iter()
                .map(|a| format!("0x{}", hex::encode(a)))
                .collect(),
        });

        info!("  Mined {} auxiliary accounts", auxiliaries.len());
    }

    let total_time = total_start.elapsed().as_secs_f64();

    // Create result structure
    let result = Create2MiningResult {
        deployer: format!("0x{}", hex::encode(deployer)),
        init_code_hash: format!("0x{}", hex::encode(init_code_hash)),
        init_code: format!("0x{}", hex::encode(init_code)),
        deploy_code: format!("0x{}", hex::encode(deploy_code)),
        storage_keys: storage_keys
            .iter()
            .map(|k| format!("0x{}", hex::encode(k)))
            .collect(),
        target_depth,
        num_contracts,
        total_time,
        contracts,
    };

    // Write to JSON file
    match serde_json::to_string_pretty(&result) {
        Ok(json) => {
            if let Err(e) = fs::write(output_path, json) {
                log::error!("Failed to write JSON: {e}");
            } else {
                info!("");
                info!("═══ CREATE2 Mining Statistics ═══");
                info!("Total contracts: {num_contracts}");
                info!("Target depth: {target_depth}");
                info!("Total auxiliary accounts: {}", num_contracts * target_depth);
                info!("Total time: {total_time:.2} seconds");
                info!(
                    "Average time per contract: {:.2} seconds",
                    total_time / num_contracts as f64
                );
                info!("Results saved to: {output_path}");
            }
        }
        Err(e) => {
            log::error!("Failed to serialize to JSON: {e}");
        }
    }
}

/// Calculate CREATE2 address
fn calculate_create2_address(
    deployer: &[u8; 20],
    salt: u32,
    init_code_hash: &[u8; 32],
) -> [u8; 20] {
    let mut data = Vec::with_capacity(85);

    // 0xff prefix
    data.push(0xff);

    // Deployer address (20 bytes)
    data.extend_from_slice(deployer);

    // Salt (32 bytes) - convert u32 to 32 bytes with zero padding
    let mut salt_bytes = [0u8; 32];
    salt_bytes[28..32].copy_from_slice(&salt.to_be_bytes());
    data.extend_from_slice(&salt_bytes);

    // Init code hash (32 bytes)
    data.extend_from_slice(init_code_hash);

    // Hash and take last 20 bytes
    let hash = keccak256(&data);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);

    address
}

/// Mine auxiliary accounts for a single contract
fn mine_auxiliaries_for_contract(
    contract_address: &[u8; 20],
    target_depth: usize,
    num_threads: usize,
) -> Vec<[u8; 20]> {
    let mut auxiliaries = Vec::new();

    // Calculate the hash of the contract address - this is the key in the account trie
    let contract_hash = keccak256(contract_address);

    for depth in 1..=target_depth {
        debug!("  Mining auxiliary at depth {depth}/{target_depth}");

        // Mine an account whose hash shares 'depth' nibbles with the contract hash
        let auxiliary = mine_account_with_hash_prefix(&contract_hash, depth, num_threads);

        debug!(
            "  Found: 0x{} (hash shares {} nibbles)",
            hex::encode(&auxiliary[..4]),
            depth
        );

        auxiliaries.push(auxiliary);
    }

    auxiliaries
}

/// Mine an account whose hash shares exactly `depth` nibbles with the target hash
fn mine_account_with_hash_prefix(
    target_hash: &[u8; 32],
    depth: usize,
    num_threads: usize,
) -> [u8; 20] {
    let result = Arc::new(Mutex::new(None));
    let found = Arc::new(Mutex::new(false));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let result_clone = Arc::clone(&result);
            let found_clone = Arc::clone(&found);
            let target_hash_copy = *target_hash;

            thread::spawn(move || {
                mine_hash_worker(
                    thread_id,
                    &target_hash_copy,
                    depth,
                    result_clone,
                    found_clone,
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    result.lock().unwrap().expect("Failed to find account")
}

/// Worker thread for hash-based mining
fn mine_hash_worker(
    thread_id: usize,
    target_hash: &[u8; 32],
    required_nibbles: usize,
    result: Arc<Mutex<Option<[u8; 20]>>>,
    found: Arc<Mutex<bool>>,
) {
    let mut rng = rand::thread_rng();
    let mut attempts = 0u64;
    const BATCH_SIZE: u64 = 1000;

    loop {
        // Check if another thread found a result
        if attempts % BATCH_SIZE == 0 && *found.lock().unwrap() {
            break;
        }

        attempts += 1;
        if attempts % 1000000 == 0 {
            debug!(
                "Thread {} (depth {}): {} million attempts",
                thread_id,
                required_nibbles,
                attempts / 1000000
            );
        }

        // Generate random address
        let mut address = [0u8; 20];
        rng.fill(&mut address);

        // Hash the address - this is how it's indexed in the account trie
        let address_hash = keccak256(&address);

        // Check if the hash matches the required prefix
        if has_hash_prefix(&address_hash, target_hash, required_nibbles) {
            let mut found_lock = found.lock().unwrap();
            if !*found_lock {
                *found_lock = true;
                let mut result_lock = result.lock().unwrap();
                *result_lock = Some(address);
                debug!("Thread {thread_id} found match after {attempts} attempts");
            }
            break;
        }
    }
}

/// Check if two hashes share the specified number of nibbles as prefix
fn has_hash_prefix(hash_a: &[u8; 32], hash_b: &[u8; 32], nibbles: usize) -> bool {
    if nibbles == 0 {
        return true;
    }

    let full_bytes = nibbles / 2;
    let has_half_byte = nibbles % 2 == 1;

    // Check full bytes must match
    for i in 0..full_bytes {
        if hash_a[i] != hash_b[i] {
            return false;
        }
    }

    // Check the half byte if needed
    if has_half_byte && full_bytes < 32 {
        let mask = 0xF0; // Check only the high nibble
        if (hash_a[full_bytes] & mask) != (hash_b[full_bytes] & mask) {
            return false;
        }
    }

    true
}

/// Compute Keccak256 hash
fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut output);
    output
}
