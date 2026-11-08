
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    error::VaultError,
    state::VaultState,
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump,
        has_one = user
    )]
    pub vault_state: Account<'info, VaultState>,

    /// CHECK: This is the System Program-owned vault PDA.
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);

    let vault_balance = ctx.accounts.vault.lamports();

    // Preserve the vault's rent-exempt minimum balance.
    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance(0);

    let available_balance = vault_balance.saturating_sub(minimum_balance);

    require!(
        amount <= available_balance,
        VaultError::InsufficientBalance
    );

    // The vault PDA must sign the System Program transfer.
    let vault_state_key = ctx.accounts.vault_state.key();

    let signer_seeds: &[&[u8]] = &[
        VAULT_SEED,
        vault_state_key.as_ref(),
        &[ctx.accounts.vault_state.vault_bump],
    ];

    let signer = &[signer_seeds];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.user.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        cpi_accounts,
        signer,
    );

    transfer(cpi_ctx, amount)?;

    msg!("Withdrawn {} lamports from the vault", amount);

    Ok(())
}
