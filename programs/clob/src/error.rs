use super::*;

#[error_code]
pub enum CLOBError {
    #[msg("Tried to become a market maker in an index that is already taken")]
    IndexAlreadyTaken,
    #[msg("This signer does not have authority over this market maker index")]
    UnauthorizedMarketMaker,
    #[msg("This market maker has insufficient balance for this limit order")]
    InsufficientBalance,
}
