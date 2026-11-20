use anchor_lang::prelude::*;

use crate::error::VaultError;

/// Виртуальные доли и виртуальные активы (ERC-4626 «virtual offset»).
///
/// Добавляются к знаменателю и числителю курса доли, чтобы атака «первого вкладчика»
/// (депозит 1 микротокена + прямой донат в `token_vault`) перестала быть выгодной:
/// донат делится с виртуальной долей, и атакующий теряет не меньше, чем отнимает у жертвы.
/// При пустом хранилище курс остаётся `1:1`.
pub const VIRTUAL_SHARES: u128 = 1;
pub const VIRTUAL_ASSETS: u128 = 1;

const Q64: u128 = 1u128 << 64;

/// Расчёт количества share-токенов при депозите (Неделя 9).
///
/// `vault_value` — полная стоимость активов хранилища в единицах `deposit_mint`
/// (см. `nav::vault_value`), а не только остаток `token_vault`.
///
/// `shares = deposit_amount * (total_shares + 1) / (vault_value + 1)` в `u128`,
/// округление вниз в пользу хранилища.
pub fn calculate_shares_to_mint(
    deposit_amount: u64,
    vault_value: u64,
    total_shares: u64,
) -> Result<u64> {
    require!(deposit_amount > 0, VaultError::ZeroAmount);

    let shares_u128 = (deposit_amount as u128)
        .checked_mul((total_shares as u128) + VIRTUAL_SHARES)
        .ok_or(VaultError::MathOverflow)?
        .checked_div((vault_value as u128) + VIRTUAL_ASSETS)
        .ok_or(VaultError::MathOverflow)?;

    let shares = u64::try_from(shares_u128).map_err(|_| VaultError::MathOverflow)?;
    require!(shares > 0, VaultError::ZeroShares);

    Ok(shares)
}

/// Расчёт количества базовых токенов к возврату при сжигании share-токенов (Неделя 10).
///
/// `tokens_out = shares_to_burn * (vault_value + 1) / (total_shares + 1)` в `u128`,
/// округление вниз. Результат никогда не превышает `vault_value`.
