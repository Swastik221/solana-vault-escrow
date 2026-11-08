
use anchor_lang::prelude::*;

use crate::{
    constants::{STATE_SEED, VAULT_SEED},
    state::VaultState,
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump
    )]
    pub vault_state: Account<'info, VaultState>,

    /// CHECK: Created manually as a System Program-owned PDA.
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>) -> Result<()> {
    let vault_state_key = ctx.accounts.vault_state.key();

    let vault_state = &mut ctx.accounts.vault_state;
    vault_state.user = ctx.accounts.user.key();
    vault_state.vault_bump = ctx.bumps.vault;
    vault_state.state_bump = ctx.bumps.vault_state;

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(0);

    let signer_seeds: &[&[u8]] = &[
        VAULT_SEED,
        vault_state_key.as_ref(),
        &[ctx.bumps.vault],
    ];

    let signer = &[signer_seeds];

    let cpi_accounts = anchor_lang::system_program::CreateAccount {
        from: ctx.accounts.user.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        cpi_accounts,
        signer,
    );
    
    anchor_lang::system_program::create_account(
        cpi_ctx,
        lamports,
        0,
        &ctx.accounts.system_program.key(),
    )?;

    msg!("Vault initialized successfully");

    Ok(())
}
