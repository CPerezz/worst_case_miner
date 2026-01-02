#[cfg(feature = "cuda")]
fn main() {
    use cc::Build;

    println!("cargo:rerun-if-changed=src/keccak_cuda.cu");

    // Fat binary: compile for multiple GPU architectures
    // Each GPU gets native code, driver picks the right one at runtime
    Build::new()
        .cuda(true)
        .file("src/keccak_cuda.cu")
        // Pascal (GTX 1080)
        .flag("-gencode=arch=compute_61,code=sm_61")
        // Turing (RTX 2080)
        .flag("-gencode=arch=compute_75,code=sm_75")
        // Ampere (RTX 3080)
        .flag("-gencode=arch=compute_86,code=sm_86")
        // Ada Lovelace (RTX 4090)
        .flag("-gencode=arch=compute_89,code=sm_89")
        // Blackwell (RTX 5090) - requires CUDA 12.8+
        // Uncomment when CUDA toolkit supports sm_120:
        // .flag("-gencode=arch=compute_120,code=sm_120")
        // PTX fallback for future architectures (JIT compiled at runtime)
        .flag("-gencode=arch=compute_89,code=compute_89")
        .flag("-O3")
        .flag("-Xptxas=-v") // Show register usage
        .compile("keccak_cuda");

    // Link CUDA runtime
    println!("cargo:rustc-link-lib=dylib=cudart");
    println!("cargo:rustc-link-search=native=/usr/local/cuda/lib64");
}

#[cfg(not(feature = "cuda"))]
fn main() {
    // Nothing to do when CUDA is not enabled
}
