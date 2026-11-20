use std::str::FromStr;

use anchor_lang::{
    error::ERROR_CODE_OFFSET,
    prelude::Pubkey,
    solana_program::instruction::Instruction,
    system_program, AccountDeserialize, AccountSerialize, InstructionData, ToAccountMetas,
};
use litesvm::LiteSVM;
use liquidity_vault::{
    accounts,
    error::VaultError,
    instruction,
    raydium::POOL_STATE_DISCRIMINATOR,
    state::{
        Strategy, UserCard, VaultState, SHARE_MINT_SEED, STRATEGY_SEED, STRATEGY_VAULT_SEED,
        USER_CARD_SEED, VAULT_SEED, VAULT_TOKEN_SEED,
    },
};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DECIMALS: u8 = 6;
const ONE_TOKEN: u64 = 1_000_000;
const SOL: u64 = 1_000_000_000;

fn pk(s: &str) -> Pubkey {
    Pubkey::from_str(s).unwrap()
}

fn token_program() -> Pubkey {
    pk("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
}

fn token_2022_program() -> Pubkey {
    pk("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
}

fn ata_program() -> Pubkey {
    pk("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
}

fn memo_program() -> Pubkey {
    pk("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
}

fn rent_sysvar() -> Pubkey {
    pk("SysvarRent111111111111111111111111111111111")
}

fn mock_clmm_program() -> Pubkey {
    pk("devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH")
}

fn custom_err(e: VaultError) -> String {
    format!("Custom({})", ERROR_CODE_OFFSET + e as u32)
}

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &liquidity_vault::id()).0
}

fn ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program().as_ref(), mint.as_ref()],
        &ata_program(),
    )
    .0
}

fn mint_bytes(decimals: u8, supply: u64, mint_authority: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[0..4].copy_from_slice(&1u32.to_le_bytes()); // COption::Some
    d[4..36].copy_from_slice(mint_authority.as_ref());
    d[36..44].copy_from_slice(&supply.to_le_bytes());
    d[44] = decimals;
    d[45] = 1; // is_initialized = true
    d
}

fn token_account_bytes(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1; // AccountState::Initialized
    d
}

fn mock_pool_state_bytes(
    amm_config: &Pubkey,
    mint_0: &Pubkey,
    mint_1: &Pubkey,
    vault_0: &Pubkey,
    vault_1: &Pubkey,
    observation: &Pubkey,
) -> Vec<u8> {
    let mut d = vec![0u8; 1544];
    d[0..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
    d[8] = 254;
    d[9..41].copy_from_slice(amm_config.as_ref());
    d[73..105].copy_from_slice(mint_0.as_ref());
    d[105..137].copy_from_slice(mint_1.as_ref());
    d[137..169].copy_from_slice(vault_0.as_ref());
    d[169..201].copy_from_slice(vault_1.as_ref());
    d[201..233].copy_from_slice(observation.as_ref());
    d[233] = DECIMALS;
    d[234] = DECIMALS;
    d[235..237].copy_from_slice(&60u16.to_le_bytes());
    d[237..253].copy_from_slice(&1_000_000_000u128.to_le_bytes());
    d[253..269].copy_from_slice(&(1u128 << 64).to_le_bytes());
    d[269..273].copy_from_slice(&0i32.to_le_bytes());
    d
}

fn set_mint(svm: &mut LiteSVM, mint: &Pubkey, decimals: u8, supply: u64, authority: &Pubkey) {
    svm.set_account(
        mint.to_bytes().into(),
        Account {
            lamports: 10_000_000,
            data: mint_bytes(decimals, supply, authority),
            owner: token_program().to_bytes().into(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn set_token_account(svm: &mut LiteSVM, address: &Pubkey, mint: &Pubkey, owner: &Pubkey, amount: u64) {
    svm.set_account(
        address.to_bytes().into(),
        Account {
            lamports: 10_000_000,
            data: token_account_bytes(mint, owner, amount),
            owner: token_program().to_bytes().into(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn read_token_amount(svm: &LiteSVM, address: &Pubkey) -> u64 {
    let acc = svm.get_account(&address.to_bytes().into()).unwrap();
    u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
}

fn read_mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let acc = svm.get_account(&mint.to_bytes().into()).unwrap();
    u64::from_le_bytes(acc.data[36..44].try_into().unwrap())
}

fn setup_svm() -> (LiteSVM, Keypair, Pubkey, Pubkey) {
    let mut svm = LiteSVM::new();
    let so_path = format!(
        "{}/../../target/deploy/liquidity_vault.so",
        env!("CARGO_MANIFEST_DIR")
    );
    let program_bytes = std::fs::read(&so_path)
        .unwrap_or_else(|_| panic!("Run `cargo build-sbf` before tests (missing {})", so_path));
    svm.add_program(liquidity_vault::id().to_bytes(), &program_bytes)
        .unwrap();

    let admin = Keypair::new();
    let admin_pk = Pubkey::new_from_array(admin.pubkey().to_bytes());
    svm.airdrop(&admin.pubkey(), 20 * SOL).unwrap();

    let deposit_mint = Pubkey::new_unique();
    let secondary_mint = Pubkey::new_unique();
    set_mint(&mut svm, &deposit_mint, DECIMALS, 0, &admin_pk);
    set_mint(&mut svm, &secondary_mint, DECIMALS, 0, &admin_pk);

    (svm, admin, deposit_mint, secondary_mint)
}

fn send_ix(svm: &mut LiteSVM, signers: &[&Keypair], ix: Instruction) -> Result<(), String> {
    let payer = signers[0].pubkey();
    let bh = svm.latest_blockhash();
    let msg = Message::new(&[ix], Some(&payer));
    let tx = Transaction::new(signers, msg, bh);
    svm.send_transaction(tx)
        .map(|_| ())
        .map_err(|e| format!("{:?}", e.err))
}

fn init_vault(svm: &mut LiteSVM, admin: &Keypair, deposit_mint: &Pubkey) -> (Pubkey, Pubkey, Pubkey) {
    let admin_pk = Pubkey::new_from_array(admin.pubkey().to_bytes());
    let vault = pda(&[VAULT_SEED, deposit_mint.as_ref()]);
    let token_vault = pda(&[VAULT_TOKEN_SEED, vault.as_ref()]);
    let share_mint = pda(&[SHARE_MINT_SEED, vault.as_ref()]);

    let ix = Instruction {
        program_id: liquidity_vault::id(),
        accounts: accounts::InitializeVault {
            admin: admin_pk,
            deposit_mint: *deposit_mint,
            vault,
            token_vault,
            share_mint,
            token_program: token_program(),
            system_program: system_program::ID,
            rent: rent_sysvar(),
        }
        .to_account_metas(None),
        data: instruction::InitializeVault {}.data(),
    };
    send_ix(svm, &[admin], ix).unwrap();
    (vault, token_vault, share_mint)
}

fn create_card(svm: &mut LiteSVM, user: &Keypair, vault: &Pubkey) -> Pubkey {
    let user_pk = Pubkey::new_from_array(user.pubkey().to_bytes());
    let user_card = pda(&[USER_CARD_SEED, vault.as_ref(), user_pk.as_ref()]);
    let ix = Instruction {
        program_id: liquidity_vault::id(),
        accounts: accounts::CreateUserCard {
            user: user_pk,
            vault: *vault,
            user_card,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::CreateUserCard {}.data(),
    };
    send_ix(svm, &[user], ix).unwrap();
    user_card
}

#[test]
fn week8_initialize_vault_and_create_user_cards() {
    let (mut svm, admin, deposit_mint, _) = setup_svm();
    let admin_pk = Pubkey::new_from_array(admin.pubkey().to_bytes());
    let (vault, token_vault, share_mint) = init_vault(&mut svm, &admin, &deposit_mint);

    let vault_acc = svm.get_account(&vault.to_bytes().into()).unwrap();
    let mut slice: &[u8] = &vault_acc.data;
    let state = VaultState::try_deserialize(&mut slice).unwrap();
    assert_eq!(state.admin, admin_pk);
    assert_eq!(state.deposit_mint, deposit_mint);
    assert_eq!(state.token_vault, token_vault);
    assert_eq!(state.share_mint, share_mint);
    assert_eq!(state.total_deposited, 0);
    assert_eq!(state.total_shares, 0);

    // Карточки двух разных пользователей имеют независимые PDA
    let user_a = Keypair::new();
    let user_b = Keypair::new();
    svm.airdrop(&user_a.pubkey(), 5 * SOL).unwrap();
    svm.airdrop(&user_b.pubkey(), 5 * SOL).unwrap();

    let card_a = create_card(&mut svm, &user_a, &vault);
    let card_b = create_card(&mut svm, &user_b, &vault);
    assert_ne!(card_a, card_b);

    let card_a_acc = svm.get_account(&card_a.to_bytes().into()).unwrap();
    let mut card_slice: &[u8] = &card_a_acc.data;
    let parsed_card = UserCard::try_deserialize(&mut card_slice).unwrap();
    assert_eq!(parsed_card.vault, vault);
    assert_eq!(
        parsed_card.owner,
        Pubkey::new_from_array(user_a.pubkey().to_bytes())
    );
    assert_eq!(parsed_card.deposited_amount, 0);
    assert_eq!(parsed_card.shares_minted, 0);
}
