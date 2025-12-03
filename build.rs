#[cfg(feature = "cuda")]
fn main() {
    use cc::Build;

    println!("cargo:rerun-if-changed=src/keccak_cuda.cu");

    Build::new()
        .cuda(true)
        .file("src/keccak_cuda.cu")
        .flag("-arch=sm_75") // Adjust based on your GPU architecture
        .flag("-O3")
        .compile("keccak_cuda");

    // Link CUDA runtime
    println!("cargo:rustc-link-lib=dylib=cudart");
    println!("cargo:rustc-link-search=native=/usr/local/cuda/lib64");
}

#[cfg(not(feature = "cuda"))]
fn main() {
    // Nothing to do when CUDA is not enabled
}