use clap::Parser;
use log::info;
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

        // Load init code if provided, otherwise use default
        let init_code = if let Some(init_code_path) = args.init_code {
            std::fs::read(&init_code_path).expect("Failed to read init code file")
        } else {
            // Default: use a simple contract bytecode (can be the compiled WorstCaseERC20)
            // For now, use a placeholder
            vec![0x60, 0x80, 0x60, 0x40, 0x52] // Basic bytecode prefix
        };

        account_miner::mine_create2_accounts(
            deployer,
            num_contracts,
            args.depth,
            args.threads,
            &init_code,
            &args.accounts_output,
        );
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

/// Parse hex address string to bytes
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
