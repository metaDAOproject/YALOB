use super::*;

use sokoban::RedBlackTree;

pub const BOOK_DEPTH: usize = 128;
pub const NUM_MARKET_MAKERS: usize = 64;

#[derive(AnchorSerialize, AnchorDeserialize)]
#[zero_copy]
pub struct OrderTree {
    pub inner: RedBlackTree<u64, Order, BOOK_DEPTH>
}


#[account]
pub struct GlobalState {
    /// The CLOB needs fees to disincentivize wash trading / TWAP manipulation.
    /// Besides, profits are virtuous :)
    pub fee_collector: Pubkey,
    pub fee_in_bps: u8,
    /// Since market maker slots are finite, we need some cost to prevent someone
    /// from taking all the market maker slots. Also, have I mentioned that profits
    /// are virtuous?
    pub market_maker_burn_in_lamports: u64,
}

#[account(zero_copy)]
pub struct OrderBook {
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub buys: OrderTree,
    pub sells: OrderTree,
    pub market_makers: [MarketMaker; NUM_MARKET_MAKERS],
}

/// To maximize cache hits and to minimize `OrderBook` size, this struct is
/// as small as possible. Many of its fields are implied rather than encoded.
/// Specifically,
/// * Whether it's a buy or sell is determined by whether it sits within `buys`
///   or `sells`.
/// * The amount of tokens that the market maker would receive if the order is
///   filled = (amount * price) / 1e9.
#[zero_copy]
#[derive(Default, AnchorSerialize, AnchorDeserialize)]
pub struct Order {
    // 24 bytes
    pub id: u32,
    pub _padding: [u8; 3],
    pub market_maker_index: u8,
    pub amount: u64,
}

#[zero_copy]
pub struct MarketMaker {
    // 48 bytes
    pub base_balance: u64,
    pub quote_balance: u64,
    pub authority: Pubkey,
}
