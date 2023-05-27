use super::*;
use std::mem::size_of;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeGlobalState<'info> {
    #[account(
        init,
        seeds = [b"WWCACOTMICMIBMHAFTTWYGHMB"],
        bump,
        payer = payer,
        space = 8 + size_of::<GlobalState>()
    )]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeOrderBook<'info> {
    pub base: Account<'info, Mint>,
    pub quote: Account<'info, Mint>,
    #[account(
        init,
        payer = payer,
        associated_token::authority = order_book,
        associated_token::mint = base
    )]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = payer,
        associated_token::authority = order_book,
        associated_token::mint = quote
    )]
    pub quote_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        seeds = [b"order_book", base.key().as_ref(), quote.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + size_of::<OrderBook>()
    )]
    pub order_book: AccountLoader<'info, OrderBook>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
