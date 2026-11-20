use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("CFvWtQbdLxeJz55CfJLbpqyqFaUFsNZPuWMU7KcPgnE2");

#[program]
pub mod liquidity_vault {
    use super::*;

    /// Неделя 8: Создание каркаса vault (настройки `VaultState`, хранилище `token_vault` и `share_mint`).
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault::initialize_vault(ctx)
    }

    /// Неделя 8: Создание персональной карточки пользователя `UserCard` на PDA.
    pub fn create_user_card(ctx: Context<CreateUserCard>) -> Result<()> {
        instructions::create_user_card::create_user_card(ctx)
    }

    /// Неделя 9: Депозит базовых токенов в vault и выпуск share-токенов пропорционально доле.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::deposit(ctx, amount)
    }

    /// Неделя 10: Вывод средств из vault через CPI со сжиганием share-токенов и возвратом доли подписью PDA.
    /// `min_amount_out` — минимальная сумма к получению (защита от изменения курса доли).
    pub fn withdraw(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<()> {
        instructions::withdraw::withdraw(ctx, shares, min_amount_out)
    }
}
