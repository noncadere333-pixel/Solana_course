use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke_signed},
};

use crate::error::VaultError;

pub const SWAP_V2_DISCRIMINATOR: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];

#[derive(Clone, Copy, Debug)]
pub struct ParsedPoolState {
    pub amm_config: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_key: Pubkey,
    pub tick_spacing: u16,
    pub sqrt_price_x64: u128,
    pub current_tick: i32,
}

pub fn parse_pool_state(pool_info: &AccountInfo, expected_program: &Pubkey) -> Result<ParsedPoolState> {
    require_keys_eq!(
        *pool_info.owner,
        *expected_program,
        VaultError::InvalidClmmProgram
    );

    let data = pool_info.try_borrow_data()?;
    require!(data.len() >= 8 + 32 + 32 + 32 + 32 + 32 + 32 + 32 + 2 + 16 + 4, VaultError::InvalidPoolState);

    let mut cursor = 8;
    let _bump = data[cursor];
    cursor += 1;
    let amm_config = Pubkey::new_from_array(data[cursor..cursor + 32].try_into().unwrap());
    cursor += 32 + 32; // owner + amm_config
    let token_mint_0 = Pubkey::new_from_array(data[cursor..cursor + 32].try_into().unwrap());
    cursor += 32;
    let token_mint_1 = Pubkey::new_from_array(data[cursor..cursor + 32].try_into().unwrap());
    cursor += 32;
    let token_vault_0 = Pubkey::new_from_array(data[cursor..cursor + 32].try_into().unwrap());
    cursor += 32;
    let token_vault_1 = Pubkey::new_from_array(data[cursor..cursor + 32].try_into().unwrap());
    cursor += 32;
    let observation_key = Pubkey::new_from_array(data[cursor..cursor + 32].try_into().unwrap());
    cursor += 32 + 1; // mint_decimals
    let tick_spacing = u16::from_le_bytes(data[cursor..cursor + 2].try_into().unwrap());
    cursor += 2;
    let _liquidity = u128::from_le_bytes(data[cursor..cursor + 16].try_into().unwrap());
    cursor += 16;
    let sqrt_price_x64 = u128::from_le_bytes(data[cursor..cursor + 16].try_into().unwrap());
    cursor += 16;
    let current_tick = i32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap());

    Ok(ParsedPoolState {
        amm_config,
        token_mint_0,
        token_mint_1,
        token_vault_0,
        token_vault_1,
        observation_key,
        tick_spacing,
        sqrt_price_x64,
        current_tick,
    })
}

pub fn invoke_swap_v2<'info>(
    clmm_program: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    amm_config: &AccountInfo<'info>,
    pool_state: &AccountInfo<'info>,
    input_token_account: &AccountInfo<'info>,
    output_token_account: &AccountInfo<'info>,
    input_vault: &AccountInfo<'info>,
    output_vault: &AccountInfo<'info>,
    observation_state: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    token_program_2022: Option<&AccountInfo<'info>>,
    memo_program: Option<&AccountInfo<'info>>,
    input_vault_mint: &AccountInfo<'info>,
    output_vault_mint: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit_x64: u128,
    is_base_input: bool,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = Vec::with_capacity(8 + 8 + 8 + 16 + 1);
    data.extend_from_slice(&SWAP_V2_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data.push(if is_base_input { 1 } else { 0 });

    let mut account_metas = vec![
        AccountMeta::new_readonly(payer.key(), true),
        AccountMeta::new_readonly(amm_config.key(), false),
        AccountMeta::new(pool_state.key(), false),
        AccountMeta::new(input_token_account.key(), false),
        AccountMeta::new(output_token_account.key(), false),
        AccountMeta::new(input_vault.key(), false),
        AccountMeta::new(output_vault.key(), false),
        AccountMeta::new(observation_state.key(), false),
        AccountMeta::new_readonly(token_program.key(), false),
    ];
    if let Some(tp22) = token_program_2022 {
        account_metas.push(AccountMeta::new_readonly(tp22.key(), false));
    }
    if let Some(memo) = memo_program {
        account_metas.push(AccountMeta::new_readonly(memo.key(), false));
    }
    account_metas.push(AccountMeta::new_readonly(input_vault_mint.key(), false));
    account_metas.push(AccountMeta::new_readonly(output_vault_mint.key(), false));

    for ra in remaining_accounts {
        account_metas.push(if ra.is_writable {
            AccountMeta::new(ra.key(), false)
        } else {
            AccountMeta::new_readonly(ra.key(), false)
        });
    }

    let ix = Instruction {
        program_id: *clmm_program.key,
        accounts: account_metas,
        data,
    };

    let mut account_infos = vec![
        payer.clone(),
        amm_config.clone(),
        pool_state.clone(),
        input_token_account.clone(),
        output_token_account.clone(),
        input_vault.clone(),
        output_vault.clone(),
        observation_state.clone(),
        token_program.clone(),
    ];
    if let Some(tp22) = token_program_2022 {
        account_infos.push(tp22.clone());
    }
    if let Some(memo) = memo_program {
        account_infos.push(memo.clone());
    }
    account_infos.push(input_vault_mint.clone());
    account_infos.push(output_vault_mint.clone());
    account_infos.extend_from_slice(remaining_accounts);

    invoke_signed(&ix, &account_infos, signer_seeds)?;
    Ok(())
}
