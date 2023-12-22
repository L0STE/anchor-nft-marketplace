use anchor_lang::error_code;

#[error_code]
pub enum MarketplaceError {
    #[msg("Not the right Token Standard")]
    InvalidTokenStandard,
    #[msg("Not the right Collection")]
    InvalidCollection,
    #[msg("Choose Another Amount")]
    InvalidAmount
}

#[error_code]
pub enum InstrospectionError {
    #[msg("Invalid Program")]
    InvalidTokenProgram,
    #[msg("Invalid Instruction")]
    InvalidIx,
    #[msg("Invalid Amount")]
    InvalidAmount,
    #[msg("Invalid Creator")]
    InvalidCreator
}