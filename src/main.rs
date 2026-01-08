use clap::Parser;
use log::info;
use std::process::Command;
use std::time::Instant;

mod account_miner;
mod storage_miner;

#[cfg(feature = "cuda")]
mod cuda_miner;

/// A mining program to create deep branches in ERC20 contract storage and account trie
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target depth for the storage/account branch
    #[arg(short, long)]
    depth: usize,

    /// Number of threads to use for mining (default: number of CPU cores)
    #[arg(short, long, default_value_t = num_cpus::get())]
    threads: usize,

    /// Use CUDA acceleration if available
    #[arg(long)]
    cuda: bool,

    /// Deployer address for CREATE2 (hex string, default: 0x0000...)
    #[arg(long)]
    deployer: Option<String>,

    /// Number of contracts to deploy via CREATE2
    #[arg(long)]
    num_contracts: Option<usize>,

    /// Path to contract init code for CREATE2 hash calculation
    #[arg(long)]
    init_code: Option<String>,

    /// Output file for CREATE2 accounts JSON
    #[arg(long, default_value = "create2_accounts.json")]
    accounts_output: String,
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

    // Mine CREATE2 accounts if requested
    if let Some(num_contracts) = args.num_contracts {
        // Parse deployer address
        let deployer = if let Some(deployer_str) = args.deployer {
            parse_address(&deployer_str).expect("Invalid deployer address")
        } else {
            [0u8; 20] // Default to zero address
        };

        // Load or generate init code, deploy code, and storage keys
        let (compiled, storage_keys): (CompiledContract, Vec<[u8; 20]>) =
            if let Some(init_code_path) = args.init_code {
                // Check if it's a .sol file or a hex file
                if init_code_path.ends_with(".sol") {
                    // Compile the Solidity file to get bytecode
                    info!("Compiling Solidity contract: {}", init_code_path);
                    let compiled =
                        compile_solidity(&init_code_path).expect("Failed to compile Solidity contract");
                    // When loading from external .sol file, we don't have storage keys
                    (compiled, Vec::new())
                } else if init_code_path.ends_with(".hex") || init_code_path.ends_with(".bin") {
                    // Read hex bytecode from file
                    info!("Loading bytecode from: {}", init_code_path);
                    let hex_content = std::fs::read_to_string(&init_code_path)
                        .expect("Failed to read bytecode file");
                    let hex_content = hex_content.trim();
                    let hex_content = hex_content.strip_prefix("0x").unwrap_or(hex_content);
                    let init_code = hex::decode(hex_content).expect("Invalid hex in bytecode file");
                    // For raw bytecode, we don't have deploy_code or storage_keys
                    (
                        CompiledContract {
                            init_code,
                            deploy_code: Vec::new(),
                        },
                        Vec::new(),
                    )
                } else {
                    // Assume it's raw bytecode
                    let init_code = std::fs::read(&init_code_path).expect("Failed to read init code file");
                    (
                        CompiledContract {
                            init_code,
                            deploy_code: Vec::new(),
                        },
                        Vec::new(),
                    )
                }
            } else if args.depth > 0 {
                // No init code provided but depth specified - generate and compile a contract with the specified depth
                info!(
                    "No init code provided. Generating contract with depth {}...",
                    args.depth
                );

                // First, mine storage slots for the contract
                let branch = storage_miner::mine_deep_branch(args.depth, args.threads, false);

                // Extract storage keys (addresses) from the mined branch
                let storage_keys: Vec<[u8; 20]> = branch.iter().map(|slot| slot.address).collect();

                // Generate the contract
                storage_miner::generate_contract(&branch);

                // Compile the generated contract
                let contract_path = "contracts/WorstCaseERC20.sol";
                info!("Compiling generated contract: {}", contract_path);
                let compiled =
                    compile_solidity(contract_path).expect("Failed to compile generated contract");

                (compiled, storage_keys)
            } else {
                panic!(
                    "For CREATE2 mining, either provide --init-code or specify --depth to auto-generate a contract"
                );
            };

        account_miner::mine_create2_accounts(
            deployer,
            num_contracts,
            args.depth,
            args.threads,
            &compiled.init_code,
            &compiled.deploy_code,
            &storage_keys,
            &args.accounts_output,
        );

        // Exit after CREATE2 mining - don't continue to storage mining
        return;
    }

    let start_time = Instant::now();

    // Mine for the deep branch (storage)
    let branch = storage_miner::mine_deep_branch(args.depth, args.threads, args.cuda);

    let elapsed = start_time.elapsed();

    // Output results
    storage_miner::print_results(&branch, elapsed.as_secs_f64());

    // Generate contract with mined storage keys
    storage_miner::generate_contract(&branch);
}

/// Result of compiling a Solidity contract
struct CompiledContract {
    /// Init code (constructor + runtime) - used for CREATE2 address calculation
    init_code: Vec<u8>,
    /// Deploy code (runtime only) - what ends up on chain
    deploy_code: Vec<u8>,
}

/// Compile a Solidity file and return both init code and runtime code
fn compile_solidity(sol_path: &str) -> Result<CompiledContract, String> {
    // Run solc to compile the contract with both --bin and --bin-runtime
    let output = Command::new("solc")
        .args([
            "--optimize",
            "--optimize-runs",
            "200",
            "--bin",
            "--bin-runtime",
            "--metadata-hash",
            "none",
            sol_path,
        ])
        .output()
        .map_err(|e| format!("Failed to run solc: {}. Make sure solc is installed.", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Solidity compilation failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    let mut init_code: Option<Vec<u8>> = None;
    let mut deploy_code: Option<Vec<u8>> = None;
    let mut next_is_binary = false;
    let mut next_is_runtime = false;

    for line in lines {
        if next_is_binary {
            let bytecode_hex = line.trim();
            if !bytecode_hex.is_empty() {
                init_code = Some(
                    hex::decode(bytecode_hex)
                        .map_err(|e| format!("Failed to decode init code hex: {}", e))?,
                );
            }
            next_is_binary = false;
        } else if next_is_runtime {
            let bytecode_hex = line.trim();
            if !bytecode_hex.is_empty() {
                deploy_code = Some(
                    hex::decode(bytecode_hex)
                        .map_err(|e| format!("Failed to decode runtime code hex: {}", e))?,
                );
            }
            next_is_runtime = false;
        }

        if line.contains("Binary:") && !line.contains("Binary of the runtime") {
            next_is_binary = true;
        } else if line.contains("Binary of the runtime") {
            next_is_runtime = true;
        }
    }

    Ok(CompiledContract {
        init_code: init_code.ok_or("Could not find init code in solc output")?,
        deploy_code: deploy_code.ok_or("Could not find runtime code in solc output")?,
    })
}

fn parse_address(hex_str: &str) -> Result<[u8; 20], String> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    if hex_str.len() != 40 {
        return Err(format!(
            "Address must be 40 hex characters, got {}",
            hex_str.len()
        ));
    }

    let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {e}"))?;

    let mut address = [0u8; 20];
    address.copy_from_slice(&bytes);
    Ok(address)
}
