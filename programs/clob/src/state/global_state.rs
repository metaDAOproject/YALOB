use super::*;

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
