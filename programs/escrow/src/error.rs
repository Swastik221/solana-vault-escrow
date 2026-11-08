use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Unauthorized access")]
    Unauthorized,

    #[msg("Invalid token amount")]
    InvalidAmount,

    #[msg("Maker and taker token accounts must be different")]
    InvalidTokenAccount,

    #[msg("Invalid token mint")]
    InvalidMint,
}