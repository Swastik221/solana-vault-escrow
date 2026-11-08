
use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, CloseAccount, Mint, Token, TokenAccount, Transfer,
};

use crate::{
    constants::ESCROW_SEED,
    error::EscrowError,
    state::Escrow,
};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        mut,
        close = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref()],
        bump = escrow.bump,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    pub mint_a: Box<Account<'info, Mint>>,
    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = taker
    )]
    pub taker_token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = maker
    )]
    pub maker_token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_a,
        token::authority = taker
    )]
    pub taker_token_a: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_take(ctx: Context<Take>) -> Result<()> {
    require_keys_neq!(
        ctx.accounts.taker.key(),
        ctx.accounts.maker.key(),
        EscrowError::Unauthorized
    );

    let receive_amount = ctx.accounts.escrow.receive;
    let offered_amount = ctx.accounts.vault.amount;

    // 1. Transfer Token B from taker to maker.
    let payment_accounts = Transfer {
        from: ctx.accounts.taker_token_b.to_account_info(),
        to: ctx.accounts.maker_token_b.to_account_info(),
        authority: ctx.accounts.taker.to_account_info(),
    };

    let payment_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        payment_accounts,
    );

    token::transfer(payment_ctx, receive_amount)?;

    // Derive the escrow PDA signer seeds.
    let maker_key = ctx.accounts.maker.key();

    let signer_seeds: &[&[u8]] = &[
        ESCROW_SEED,
        maker_key.as_ref(),
        &[ctx.accounts.escrow.bump],
    ];

    let signer = &[signer_seeds];

    // 2. Transfer all deposited Token A from vault to taker.
    let token_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.taker_token_a.to_account_info(),
        authority: ctx.accounts.escrow.to_account_info(),
    };

    let token_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        token_accounts,
        signer,
    );

    token::transfer(token_ctx, offered_amount)?;

    // 3. Close the empty vault and return its rent to maker.
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

    msg!("Escrow completed successfully");
    msg!("Token B paid: {}", receive_amount);
    msg!("Token A received: {}", offered_amount);

    Ok(())
}
