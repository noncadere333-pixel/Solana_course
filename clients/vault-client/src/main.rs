pub mod raydium;
pub mod rpc;
pub mod vault;

use std::{env, fs, path::PathBuf, str::FromStr};

use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, bail, Context, Result};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::rpc::RpcClient;

/// Адреса развёрнутых артефактов в Solana Devnet по умолчанию (обновляются после деплоя).
pub const DEFAULT_DEPOSIT_MINT_DEVNET: &str = "wKGeYnbJWf2iqakmqFkYGcnmWHQ2roJ6jJ1664Zf2oR";
pub const DEFAULT_SECONDARY_MINT_DEVNET: &str = "EWXdK61y5MT8HfZypYJBLNj2A2paRAc5pXojcpeFkRwu";
pub const DEFAULT_POOL_STATE_DEVNET: &str = "DKRQA3X9R6oneVnfgXEtCZdQmPRyASYV6rLXv7dBBD3n";

fn load_keypair(custom_path: Option<&str>) -> Result<Keypair> {
    let candidates: Vec<PathBuf> = if let Some(p) = custom_path {
        vec![PathBuf::from(p)]
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        vec![
            PathBuf::from("../.keys/id.json"),
            PathBuf::from(".keys/id.json"),
            PathBuf::from(format!("{}/.config/solana/id.json", home)),
        ]
    };

    for path in &candidates {
        if path.exists() {
            let text = fs::read_to_string(path)
                .with_context(|| format!("Failed to read keypair file {}", path.display()))?;
            let bytes: Vec<u8> = serde_json::from_str(&text)
                .with_context(|| format!("Invalid JSON in keypair file {}", path.display()))?;
            let arr: [u8; 64] = bytes
                .try_into()
                .map_err(|_| anyhow!("Keypair in {} is not 64 bytes", path.display()))?;
            let secret: [u8; 32] = arr[0..32]
                .try_into()
                .map_err(|_| anyhow!("Invalid secret key slice in {}", path.display()))?;
            return Ok(Keypair::new_from_array(secret));
        }
    }

    bail!(
        "No Solana keypair found in {:?}",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    )
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|idx| args.get(idx + 1).cloned())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn print_help() {
    println!(
        "vault-client — Rust CLI for Liquidity Vault & Raydium CLMM (Weeks 8–13)

USAGE:
  cargo run -p vault-client -- <COMMAND> [OPTIONS]

GLOBAL OPTIONS:
  --cluster <devnet|localnet>   Target Solana cluster (default: devnet)
  --rpc-url <URL>               Custom RPC endpoint URL
  --keypair <PATH>              Wallet keypair path (default: ../.keys/id.json or ~/.config/solana/id.json)
  --mint <PUBKEY>               Base deposit token mint of the vault

COMMANDS:
  Week 8 (Vault Skeleton & PDAs):
    init                        Initialize VaultState, token_vault, and share_mint PDAs
    card                        Create personal UserCard PDA for the wallet
    show                        Show VaultState, UserCard, wallet balances

  Week 9 (Deposit & Share Tokens):
    deposit --amount <UI_AMT>   Deposit tokens into vault and mint proportional share tokens"
    );
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || has_flag(&args, "--help") || has_flag(&args, "-h") {
        print_help();
        return Ok(());
    }

    let command = args[0].as_str();
    let cluster = parse_flag_value(&args, "--cluster").unwrap_or_else(|| "devnet".to_string());
    let rpc_url = parse_flag_value(&args, "--rpc-url");
    let keypair_path = parse_flag_value(&args, "--keypair");

    let rpc = RpcClient::new(&cluster, rpc_url.as_deref());
    let genesis = rpc.verify_cluster_genesis()?;
    let wallet = load_keypair(keypair_path.as_deref())?;
    let wallet_pk = Pubkey::new_from_array(wallet.pubkey().to_bytes());
    let sol_balance = rpc.get_balance(&wallet_pk)?;

    println!(
        "[RPC] cluster={} genesis={} wallet={} balance={:.4} SOL",
        rpc.cluster,
        genesis,
        wallet_pk,
        (sol_balance as f64) / 1_000_000_000.0
    );

    let resolve_deposit_mint = || -> Result<Pubkey> {
        let s = parse_flag_value(&args, "--mint")
            .unwrap_or_else(|| DEFAULT_DEPOSIT_MINT_DEVNET.to_string());
        Pubkey::from_str(&s).with_context(|| format!("Invalid --mint pubkey: {}", s))
    };

    let resolve_secondary_mint = || -> Result<Pubkey> {
        let s = parse_flag_value(&args, "--secondary-mint")
            .unwrap_or_else(|| DEFAULT_SECONDARY_MINT_DEVNET.to_string());
        Pubkey::from_str(&s).with_context(|| format!("Invalid --secondary-mint pubkey: {}", s))
    };

    let resolve_pool = || -> Result<Pubkey> {
        let s = parse_flag_value(&args, "--pool")
            .unwrap_or_else(|| DEFAULT_POOL_STATE_DEVNET.to_string());
        Pubkey::from_str(&s).with_context(|| format!("Invalid --pool pubkey: {}", s))
    };

    match command {
        "init" => {
            let mint = resolve_deposit_mint()?;
            vault::cmd_init_vault(&rpc, &wallet, &mint)?;
        }
        "card" => {
            let mint = resolve_deposit_mint()?;
            vault::cmd_create_card(&rpc, &wallet, &mint)?;
        }
        "show" => {
            let mint = resolve_deposit_mint()?;
            vault::cmd_show_vault(&rpc, &wallet, &mint)?;
        }
        "deposit" => {
            let mint = resolve_deposit_mint()?;
            let amount = parse_flag_value(&args, "--amount")
                .ok_or_else(|| anyhow!("Missing --amount <UI_AMOUNT> for deposit"))?;
            vault::cmd_deposit(&rpc, &wallet, &mint, &amount)?;
        }
        cmd => bail!("Unknown command: {}. Run with --help for usage.", cmd),
    }

    Ok(())
}
