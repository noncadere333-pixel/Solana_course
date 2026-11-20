use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod math;
pub mod nav;
pub mod raydium;
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

    /// Неделя 12: Привязка стратегии Raydium CLMM и создание второго хранилища `secondary_vault`.
    pub fn init_strategy(ctx: Context<InitStrategy>) -> Result<()> {
        instructions::init_strategy::init_strategy(ctx)
    }

    /// Неделя 12: Обмен токенов из vault через пул Raydium CLMM (`swap_v2`) с проверкой `min_amount_out`.
    pub fn swap<'info>(
        ctx: Context<'info, SwapVault<'info>>,
        amount_in: u64,
        min_amount_out: u64,
        sqrt_price_limit_x64: u128,
        a_to_b: bool,
    ) -> Result<()> {
        instructions::swap::swap(ctx, amount_in, min_amount_out, sqrt_price_limit_x64, a_to_b)
    }
}
