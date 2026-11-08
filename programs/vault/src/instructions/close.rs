
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    state::VaultState,
};

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump,
        has_one = user,
        close = user
    )]
    pub vault_state: Account<'info, VaultState>,

    /// CHECK: System Program-owned vault PDA whose entire balance is returned.
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_close(ctx: Context<Close>) -> Result<()> {
    let vault_state_key = ctx.accounts.vault_state.key();
    let vault_bump = ctx.accounts.vault_state.vault_bump;

    // Read the entire vault balance, including its rent-exempt reserve.
    let amount = ctx.accounts.vault.lamports();

    let signer_seeds: &[&[u8]] = &[
        VAULT_SEED,
        vault_state_key.as_ref(),
        &[vault_bump],
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

    msg!("Vault closed. Returned {} lamports to user", amount);

    Ok(())
}
