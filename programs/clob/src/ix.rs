use super::*;
use std::mem::size_of;

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
    #[account(
        init,
        seeds = [b"order_book"],
        bump,
        payer = payer,
        space = 8 + size_of::<OrderBook>()
    )]
    pub order_book: AccountLoader<'info, OrderBook>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
