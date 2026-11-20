use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Unauthorized caller for this vault operation")]
    Unauthorized,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
}
