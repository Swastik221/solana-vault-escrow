
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    error::VaultError,
    state::VaultState,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump,
        has_one = user
    )]
    pub vault_state: Account<'info, VaultState>,

    /// CHECK: This is a System Program-owned PDA used to hold SOL.
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);

    let cpi_accounts = Transfer {
        from: ctx.accounts.user.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
      ctx.accounts.system_program.key(),
      cpi_accounts,
  );

    transfer(cpi_ctx, amount)?;

    msg!("Deposited {} lamports into the vault", amount);

    Ok(())
}

