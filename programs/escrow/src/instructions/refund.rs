
use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, CloseAccount, Mint, Token, TokenAccount, Transfer,
};

use crate::{
  constants::ESCROW_SEED,
  state::Escrow,
};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        close = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref()],
        bump = escrow.bump,
        has_one = maker,
        has_one = mint_a
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    pub mint_a: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_a,
        token::authority = maker
    )]
    pub maker_token_a: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_refund(ctx: Context<Refund>) -> Result<()> {
    let amount = ctx.accounts.vault.amount;

    let maker_key = ctx.accounts.maker.key();

    let signer_seeds: &[&[u8]] = &[
        ESCROW_SEED,
        maker_key.as_ref(),
        &[ctx.accounts.escrow.bump],
    ];

    let signer = &[signer_seeds];

    // 1. Return all Token A to the maker.
    let transfer_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.maker_token_a.to_account_info(),
        authority: ctx.accounts.escrow.to_account_info(),
    };

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        transfer_accounts,
        signer,
    );

    token::transfer(transfer_ctx, amount)?;

    // 2. Close the empty vault and return its rent to maker.
    let close_accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.maker.to_account_info(),
        authority: ctx.accounts.escrow.to_account_info(),
    };

    let close_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        close_accounts,
        signer,
    );

    token::close_account(close_ctx)?;

    msg!("Escrow refunded successfully");
    msg!("Token A returned: {}", amount);

    Ok(())
}
