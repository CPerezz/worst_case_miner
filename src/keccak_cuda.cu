// CUDA Keccak256 implementation for storage slot mining
// Optimized for finding addresses with specific prefix patterns

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>

// xorshift64* PRNG - better statistical properties than LCG
__device__ inline uint64_t xorshift64star(uint64_t* state) {
    uint64_t x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    return x * 0x2545F4914F6CDD1DULL;
}

// Keccak round constants
__constant__ uint64_t keccak_round_constants[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL,
    0x800000000000808aULL, 0x8000000080008000ULL,
    0x000000000000808bULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL,
    0x000000000000008aULL, 0x0000000000000088ULL,
    0x0000000080008009ULL, 0x000000008000000aULL,
    0x000000008000808bULL, 0x800000000000008bULL,
    0x8000000000008089ULL, 0x8000000000008003ULL,
    0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800aULL, 0x800000008000000aULL,
    0x8000000080008081ULL, 0x8000000000008080ULL,
    0x0000000080000001ULL, 0x8000000080008008ULL
};

// 64-bit rotate left
__device__ inline uint64_t rotl64(uint64_t x, int n) {
    return (x << n) | (x >> (64 - n));
}

// Keccak-f[1600] permutation
__device__ void keccak_f1600(uint64_t state[25]) {
    uint64_t B[25];
    uint64_t C[5], D[5];

    #pragma unroll
    for (int round = 0; round < 24; round++) {
        // Theta
        #pragma unroll
        for (int i = 0; i < 5; i++) {
            C[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }

        #pragma unroll
        for (int i = 0; i < 5; i++) {
            D[i] = C[(i + 4) % 5] ^ rotl64(C[(i + 1) % 5], 1);
        }

        #pragma unroll
        for (int i = 0; i < 5; i++) {
            #pragma unroll
            for (int j = 0; j < 25; j += 5) {
                state[i + j] ^= D[i];
            }
        }

        // Rho and Pi - hard-coded for immediate rotate values
        // Eliminates constant memory lookups and index computation
        B[0]  = state[0];
        B[10] = rotl64(state[1],  1);
        B[20] = rotl64(state[2], 62);
        B[5]  = rotl64(state[3], 28);
        B[15] = rotl64(state[4], 27);
        B[16] = rotl64(state[5], 36);
        B[1]  = rotl64(state[6], 44);
        B[11] = rotl64(state[7],  6);
        B[21] = rotl64(state[8], 55);
        B[6]  = rotl64(state[9], 20);
        B[7]  = rotl64(state[10], 3);
        B[17] = rotl64(state[11],10);
        B[2]  = rotl64(state[12],43);
        B[12] = rotl64(state[13],25);
        B[22] = rotl64(state[14],39);
        B[23] = rotl64(state[15],41);
        B[8]  = rotl64(state[16],45);
        B[18] = rotl64(state[17],15);
        B[3]  = rotl64(state[18],21);
        B[13] = rotl64(state[19], 8);
        B[14] = rotl64(state[20],18);
        B[24] = rotl64(state[21], 2);
        B[9]  = rotl64(state[22],61);
        B[19] = rotl64(state[23],56);
        B[4]  = rotl64(state[24],14);

        // Chi
        #pragma unroll
        for (int j = 0; j < 25; j += 5) {
            uint64_t t[5];
            #pragma unroll
            for (int i = 0; i < 5; i++) {
                t[i] = B[i + j];
            }
            #pragma unroll
            for (int i = 0; i < 5; i++) {
                state[i + j] = t[i] ^ ((~t[(i + 1) % 5]) & t[(i + 2) % 5]);
            }
        }

        // Iota
        state[0] ^= keccak_round_constants[round];
    }
}

// Calculate storage slot for an address
__device__ void calculate_storage_slot(uint8_t address[20], uint64_t base_slot, uint8_t output[32]) {
    uint64_t state[25] = {0};

    // Prepare input: padded address (32 bytes) + slot (32 bytes)
    uint8_t input[64];

    // Pad address to 32 bytes
    for (int i = 0; i < 12; i++) input[i] = 0;
    for (int i = 0; i < 20; i++) input[12 + i] = address[i];

    // Add slot (big-endian)
    for (int i = 0; i < 24; i++) input[32 + i] = 0;
    for (int i = 0; i < 8; i++) {
        input[32 + 24 + i] = (base_slot >> (56 - i * 8)) & 0xFF;
    }

    // Load input into state (little-endian)
    for (int i = 0; i < 8; i++) {
        state[i] = 0;
        for (int j = 0; j < 8; j++) {
            state[i] |= ((uint64_t)input[i * 8 + j]) << (j * 8);
        }
    }

    // Add padding
    state[8] = 0x01;
    state[16] = 0x8000000000000000ULL;

    // Apply Keccak-f[1600]
    keccak_f1600(state);

    // Extract output (first 32 bytes)
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 8; j++) {
            output[i * 8 + j] = (state[i] >> (j * 8)) & 0xFF;
        }
    }
}

// Check if two byte arrays share a prefix of n nibbles
// Optimized with early rejection using word-level comparisons
__device__ bool check_nibble_prefix(const uint8_t* a, const uint8_t* b, int nibbles) {
    int full_bytes = nibbles / 2;
    bool has_half = (nibbles % 2) == 1;

    // Fast path: compare first 4 bytes as uint32 if we need 8+ nibbles
    if (nibbles >= 8) {
        if (*(const uint32_t*)a != *(const uint32_t*)b) return false;
        // If we need exactly 8 nibbles, we're done
        if (nibbles == 8) return true;
    }

    // Fast path: compare first 8 bytes as uint64 if we need 16+ nibbles
    if (nibbles >= 16) {
        if (*(const uint64_t*)a != *(const uint64_t*)b) return false;
        // If we need exactly 16 nibbles, we're done
        if (nibbles == 16) return true;
    }

    // Handle remaining bytes
    int start_byte = (nibbles >= 16) ? 8 : ((nibbles >= 8) ? 4 : 0);
    for (int i = start_byte; i < full_bytes; i++) {
        if (a[i] != b[i]) return false;
    }

    if (has_half && full_bytes < 32) {
        if ((a[full_bytes] & 0xF0) != (b[full_bytes] & 0xF0)) return false;
    }

    return true;
}

// CUDA kernel for mining addresses with specific storage key prefixes
__global__ void mine_storage_slots(
    uint8_t* target_prefix,      // Target storage key prefix to match
    int required_nibbles,         // Number of nibbles that must match
    uint64_t base_slot,          // ERC20 balance mapping slot (usually 0)
    uint64_t start_nonce,        // Starting nonce for this kernel
    uint64_t max_attempts,       // Maximum attempts per thread
    uint8_t* result_address,     // Output: found address (20 bytes)
    uint8_t* result_storage_key, // Output: storage key (32 bytes)
    int* found                   // Output: 1 if found, 0 otherwise
) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t nonce = start_nonce + tid * max_attempts;

    // Generate random addresses using nonce as seed
    for (uint64_t attempt = 0; attempt < max_attempts && *found == 0; attempt++) {
        uint8_t address[20];
        uint8_t storage_key[32];

        // Generate pseudo-random address using xorshift64*
        // Initialize state with nonce+attempt, ensuring it's never 0
        uint64_t state = nonce + attempt + 1;
        #pragma unroll
        for (int i = 0; i < 20; i += 8) {
            uint64_t rand = xorshift64star(&state);
            // Extract up to 8 bytes from the 64-bit random number
            for (int j = 0; j < 8 && (i + j) < 20; j++) {
                address[i + j] = (rand >> (j * 8)) & 0xFF;
            }
        }

        // Calculate storage slot
        calculate_storage_slot(address, base_slot, storage_key);

        // Check if it matches the required prefix
        if (check_nibble_prefix(storage_key, target_prefix, required_nibbles)) {
            // Use atomic compare-and-swap to ensure only one thread wins
            int old = atomicCAS(found, 0, 1);
            if (old == 0) {
                // We won! Copy results
                for (int i = 0; i < 20; i++) {
                    result_address[i] = address[i];
                }
                for (int i = 0; i < 32; i++) {
                    result_storage_key[i] = storage_key[i];
                }
            }
            return;
        }
    }
}

// Verification function to test CUDA keccak against CPU
extern "C" {
    void cuda_verify_keccak(
        uint8_t* test_address,  // 20 bytes
        uint64_t base_slot,
        uint8_t* result_storage_key  // 32 bytes output
    ) {
        uint8_t *d_addr, *d_result;
        cudaMalloc(&d_addr, 20);
        cudaMalloc(&d_result, 32);
        cudaMemcpy(d_addr, test_address, 20, cudaMemcpyHostToDevice);

        // Launch single-thread kernel to compute storage slot
        extern __global__ void verify_keccak_kernel(uint8_t* addr, uint64_t slot, uint8_t* result);
        verify_keccak_kernel<<<1, 1>>>(d_addr, base_slot, d_result);
        cudaDeviceSynchronize();

        cudaMemcpy(result_storage_key, d_result, 32, cudaMemcpyDeviceToHost);
        cudaFree(d_addr);
        cudaFree(d_result);
    }

    // Debug function to get a generated address and its storage key from the PRNG
    void cuda_debug_prng(
        uint64_t seed,
        uint64_t base_slot,
        uint8_t* result_address,     // 20 bytes output
        uint8_t* result_storage_key  // 32 bytes output
    ) {
        uint8_t *d_addr, *d_key;
        cudaMalloc(&d_addr, 20);
        cudaMalloc(&d_key, 32);

        extern __global__ void debug_prng_kernel(uint64_t seed, uint64_t slot, uint8_t* addr, uint8_t* key);
        debug_prng_kernel<<<1, 1>>>(seed, base_slot, d_addr, d_key);
        cudaDeviceSynchronize();

        cudaMemcpy(result_address, d_addr, 20, cudaMemcpyDeviceToHost);
        cudaMemcpy(result_storage_key, d_key, 32, cudaMemcpyDeviceToHost);
        cudaFree(d_addr);
        cudaFree(d_key);
    }
}

__global__ void verify_keccak_kernel(uint8_t* addr, uint64_t slot, uint8_t* result) {
    uint8_t address[20];
    uint8_t storage_key[32];
    for (int i = 0; i < 20; i++) address[i] = addr[i];
    calculate_storage_slot(address, slot, storage_key);
    for (int i = 0; i < 32; i++) result[i] = storage_key[i];
}

__global__ void debug_prng_kernel(uint64_t seed, uint64_t slot, uint8_t* result_addr, uint8_t* result_key) {
    uint8_t address[20];
    uint8_t storage_key[32];

    // Generate address using same PRNG as mining kernel (xorshift64*)
    uint64_t state = seed + 1;  // Ensure never 0
    for (int i = 0; i < 20; i += 8) {
        uint64_t rand = xorshift64star(&state);
        for (int j = 0; j < 8 && (i + j) < 20; j++) {
            address[i + j] = (rand >> (j * 8)) & 0xFF;
        }
    }

    calculate_storage_slot(address, slot, storage_key);

    for (int i = 0; i < 20; i++) result_addr[i] = address[i];
    for (int i = 0; i < 32; i++) result_key[i] = storage_key[i];
}

// Helper macro for CUDA error checking
#define CUDA_CHECK(call) do { \
    cudaError_t err = call; \
    if (err != cudaSuccess) { \
        fprintf(stderr, "CUDA error at %s:%d: %s\n", __FILE__, __LINE__, cudaGetErrorString(err)); \
        *found = false; \
        return; \
    } \
} while(0)

// Get SM count for optimal grid sizing
extern "C" {
    int cuda_get_sm_count() {
        int device;
        cudaGetDevice(&device);
        int sm_count;
        cudaDeviceGetAttribute(&sm_count, cudaDevAttrMultiProcessorCount, device);
        return sm_count;
    }
}

// C interface for Rust FFI
extern "C" {
    void cuda_mine_storage_slot(
        uint8_t* target_prefix,
        int required_nibbles,
        uint64_t base_slot,
        uint8_t* result_address,
        uint8_t* result_storage_key,
        bool* found,
        int blocks,
        int threads_per_block,
        uint64_t attempts_per_thread,
        uint64_t start_nonce
    ) {
        // Allocate device memory
        uint8_t *d_target, *d_result_addr, *d_result_key;
        int *d_found;

        CUDA_CHECK(cudaMalloc(&d_target, 32));
        CUDA_CHECK(cudaMalloc(&d_result_addr, 20));
        CUDA_CHECK(cudaMalloc(&d_result_key, 32));
        CUDA_CHECK(cudaMalloc(&d_found, sizeof(int)));

        // Copy input to device
        CUDA_CHECK(cudaMemcpy(d_target, target_prefix, 32, cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemset(d_found, 0, sizeof(int)));

        // Launch kernel
        mine_storage_slots<<<blocks, threads_per_block>>>(
            d_target,
            required_nibbles,
            base_slot,
            start_nonce,
            attempts_per_thread,
            d_result_addr,
            d_result_key,
            d_found
        );

        // Check for launch errors
        CUDA_CHECK(cudaGetLastError());

        // Wait for completion
        CUDA_CHECK(cudaDeviceSynchronize());

        // Copy results back
        int found_flag;
        CUDA_CHECK(cudaMemcpy(&found_flag, d_found, sizeof(int), cudaMemcpyDeviceToHost));

        if (found_flag) {
            CUDA_CHECK(cudaMemcpy(result_address, d_result_addr, 20, cudaMemcpyDeviceToHost));
            CUDA_CHECK(cudaMemcpy(result_storage_key, d_result_key, 32, cudaMemcpyDeviceToHost));
            *found = true;
        } else {
            *found = false;
        }

        // Clean up
        cudaFree(d_target);
        cudaFree(d_result_addr);
        cudaFree(d_result_key);
        cudaFree(d_found);
    }
}