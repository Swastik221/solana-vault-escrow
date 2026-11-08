pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("T89b167ssQxwDN8W4o19Xk3mjdDSWoMpMemL3HjHNm5");

#[program]
pub mod escrow {
    use super::*;

    pub fn make(
        ctx: Context<Make>,
        amount: u64,
        receive: u64,
    ) -> Result<()> {
        instructions::make::handle_make(ctx, amount, receive)
    }
    pub fn take(ctx: Context<Take>) -> Result<()> {
        instructions::take::handle_take(ctx)
    }
}