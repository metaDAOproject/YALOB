use super::*;

pub const BOOK_DEPTH: usize = 128;
pub const NUM_MARKET_MAKERS: usize = 64;

#[account]
pub struct GlobalState {
    /// The CLOB needs fees to disincentivize wash trading / TWAP manipulation.
    /// Besides, profits are virtuous :)
    pub fee_admin: Pubkey,
    pub fee_in_bps: u8,
}

#[account(zero_copy)]
pub struct OrderBook {
    pub global_state: Pubkey,
    pub buys: [Order; BOOK_DEPTH],
    pub sells: [Order; BOOK_DEPTH],
    pub market_makers: [MarketMaker; NUM_MARKET_MAKERS],
}

/// To maximize cache hits and to minimize `OrderBook` size, this struct is
/// as small as possible. Many of its fields are 'implied' rather than encoded.
/// Specifically,
/// * Whether it's a buy or sell is determined by whether it sits within `buys`
///   or `sells`.
/// * The amount of tokens that the market maker would receive if the order is
///   filled = (amount * price) / 1e9.
#[zero_copy]
pub struct Order {
    // 24 bytes
    pub id: u32,
    pub _padding: [u8; 3],
    pub market_maker_index: u8,
    pub amount: u64,
    pub price: u64,
}

#[zero_copy]
pub struct MarketMaker {
    // 48 bytes
    pub base_amount_owed: u64,
    pub quote_amount_owed: u64,
    pub authority: Pubkey,
}
