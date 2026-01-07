// CUDA-accelerated mining module
use log::{info, warn};

#[cfg(feature = "cuda")]
use crate::storage_miner::{calculate_storage_slot, has_nibble_prefix};

#[cfg(feature = "cuda")]
unsafe extern "C" {
    fn cuda_mine_storage_slot(
        target_prefix: *const u8,
        required_nibbles: i32,
        base_slot: u64,
        result_address: *mut u8,
        result_storage_key: *mut u8,
        found: *mut bool,
        blocks: i32,
        threads_per_block: i32,
        attempts_per_thread: u64,
        start_nonce: u64,
    );
}

#[cfg(feature = "cuda")]
pub fn mine_with_cuda(
    target_prefix: &[u8; 32],
    required_nibbles: usize,
    base_slot: u64,
) -> Option<([u8; 20], [u8; 32])> {
    let mut result_address = [0u8; 20];
    let mut result_storage_key = [0u8; 32];
    let mut found = false;

    // CUDA configuration - 256 blocks is empirically optimal for this kernel
    // Testing showed that scaling with SM count (e.g., 510 or 1360 blocks)
    // causes 50%+ slowdown, likely due to memory contention on the found flag
    // and wasted work after a match is found
    let blocks = 256;
    let threads_per_block = 256;

    // Scale attempts based on required nibbles
    // Each nibble adds 4 bits of difficulty (16x harder)
    // Base: 100k attempts, scale up for higher nibble counts
    let attempts_per_thread: u64 = match required_nibbles {
        0..=3 => 1_000,       // Very easy, just for testing
        4..=5 => 10_000,
        6 => 100_000,
        7 => 1_000_000,
        8 => 10_000_000,
        9 => 50_000_000,
        10 => 100_000_000,
        11 => 200_000_000,
        12 => 500_000_000,
        _ => 1_000_000_000,
    };

    // Calculate max iterations to attempt before giving up
    // For very high nibble counts, we may need multiple kernel launches
    // Each nibble requires 16x more work on average
    let max_iterations = match required_nibbles {
        0..=7 => 1,
        8 => 5,
        9 => 20,
        10 => 50,
        11 => 100,
        12 => 500,
        _ => 2000,
    };

    let total_attempts_per_iteration = blocks as u64 * threads_per_block as u64 * attempts_per_thread;

    info!(
        "Mining with CUDA: {} blocks, {} threads/block, {} attempts/thread ({:.2}B attempts/iteration, max {} iterations)",
        blocks, threads_per_block, attempts_per_thread,
        total_attempts_per_iteration as f64 / 1_000_000_000.0,
        max_iterations
    );
    info!(
        "Target prefix (first {} nibbles): 0x{}",
        required_nibbles,
        hex::encode(&target_prefix[..required_nibbles.div_ceil(2)])
    );

    // Calculate attempts per iteration to compute start_nonce for each iteration
    let attempts_per_iteration = blocks as u64 * threads_per_block as u64 * attempts_per_thread;

    for iteration in 0..max_iterations {
        if iteration > 0 && iteration % 10 == 0 {
            info!("CUDA iteration {}/{}", iteration, max_iterations);
        }

        // Each iteration starts where the previous one left off
        let start_nonce = iteration as u64 * attempts_per_iteration;

        unsafe {
            cuda_mine_storage_slot(
                target_prefix.as_ptr(),
                required_nibbles as i32,
                base_slot,
                result_address.as_mut_ptr(),
                result_storage_key.as_mut_ptr(),
                &mut found as *mut bool,
                blocks,
                threads_per_block,
                attempts_per_thread,
                start_nonce,
            );
        }

        if found {
            // Verify the result using CPU to catch any CUDA false positives
            let cpu_storage_key = calculate_storage_slot(&result_address, base_slot);
            if !has_nibble_prefix(&cpu_storage_key, target_prefix, required_nibbles) {
                warn!(
                    "CUDA returned false positive! Address 0x{} does not match {} nibbles. Continuing search...",
                    hex::encode(&result_address),
                    required_nibbles
                );
                // Reset found flag and continue searching
                found = false;
                continue;
            }

            if iteration > 0 {
                info!("CUDA found match on iteration {}", iteration + 1);
            }
            return Some((result_address, cpu_storage_key));
        }
    }

    None
}

#[cfg(not(feature = "cuda"))]
pub fn mine_with_cuda(
    _target_prefix: &[u8; 32],
    _required_nibbles: usize,
    _base_slot: u64,
) -> Option<([u8; 20], [u8; 32])> {
    panic!("CUDA support not enabled. Build with --features cuda");
}

/// Check if CUDA is available
pub fn cuda_available() -> bool {
    #[cfg(feature = "cuda")]
    {
        // In a real implementation, we'd check if CUDA runtime is available
        true
    }
    #[cfg(not(feature = "cuda"))]
    {
        false
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use crate::storage_miner::calculate_storage_slot;

    // Test-only FFI bindings
    unsafe extern "C" {
        fn cuda_verify_keccak(
            test_address: *const u8,
            base_slot: u64,
            result_storage_key: *mut u8,
        );

        fn cuda_debug_prng(
            seed: u64,
            base_slot: u64,
            result_address: *mut u8,
            result_storage_key: *mut u8,
        );
    }

    fn verify_cuda_keccak(address: &[u8; 20], base_slot: u64) -> [u8; 32] {
        let mut result = [0u8; 32];
        unsafe {
            cuda_verify_keccak(address.as_ptr(), base_slot, result.as_mut_ptr());
        }
        result
    }

    fn debug_cuda_prng(seed: u64, base_slot: u64) -> ([u8; 20], [u8; 32]) {
        let mut address = [0u8; 20];
        let mut storage_key = [0u8; 32];
        unsafe {
            cuda_debug_prng(seed, base_slot, address.as_mut_ptr(), storage_key.as_mut_ptr());
        }
        (address, storage_key)
    }

    /// Verify CUDA keccak implementation matches CPU implementation
    #[test]
    fn test_cuda_keccak_matches_cpu() {
        let test_addr: [u8; 20] = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22,
            0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
        ];

        let cpu_result = calculate_storage_slot(&test_addr, 0);
        let cuda_result = verify_cuda_keccak(&test_addr, 0);

        assert_eq!(
            cpu_result, cuda_result,
            "CUDA keccak mismatch!\nCPU:  0x{}\nCUDA: 0x{}",
            hex::encode(&cpu_result),
            hex::encode(&cuda_result)
        );
    }

    /// Verify CUDA PRNG generates addresses that produce correct storage keys
    #[test]
    fn test_cuda_prng_produces_valid_keys() {
        // Test multiple seeds to ensure consistency
        for seed in [0u64, 1, 12345, 999999, u64::MAX - 1] {
            let (prng_addr, cuda_key) = debug_cuda_prng(seed, 0);
            let cpu_key = calculate_storage_slot(&prng_addr, 0);

            assert_eq!(
                cuda_key, cpu_key,
                "CUDA PRNG key mismatch for seed {}!\nAddress: 0x{}\nCUDA key: 0x{}\nCPU key:  0x{}",
                seed,
                hex::encode(&prng_addr),
                hex::encode(&cuda_key),
                hex::encode(&cpu_key)
            );
        }
    }

    /// Verify CUDA keccak works with different base slots
    #[test]
    fn test_cuda_keccak_different_slots() {
        let test_addr: [u8; 20] = [0xaa; 20];

        for slot in [0u64, 1, 2, 100, u64::MAX] {
            let cpu_result = calculate_storage_slot(&test_addr, slot);
            let cuda_result = verify_cuda_keccak(&test_addr, slot);

            assert_eq!(
                cpu_result, cuda_result,
                "CUDA keccak mismatch for slot {}!\nCPU:  0x{}\nCUDA: 0x{}",
                slot,
                hex::encode(&cpu_result),
                hex::encode(&cuda_result)
            );
        }
    }
}
