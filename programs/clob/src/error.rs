use super::*;

#[error_code]
pub enum CLOBError {
    #[msg("Tried to become a market maker in an index that is already taken")]
    IndexAlreadyTaken,
}
