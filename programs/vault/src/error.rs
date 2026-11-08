use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Unauthorized access")]
    Unauthorized,

    #[msg("Invalid deposit amount")]
    InvalidAmount,

    #[msg("Insufficient vault balance")]
    InsufficientBalance,

    #[msg("Vault balance cannot be reduced below rent-exempt minimum")]
    RentExemptionViolation,
}