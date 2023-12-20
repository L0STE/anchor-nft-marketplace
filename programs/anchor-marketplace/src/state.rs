use anchor_lang::prelude::*;

#[account]
pub struct Marketplace {
    pub admin: Pubkey,
    pub fee: u16,
    pub name: String,
}

impl Space for Marketplace {
    const INIT_SPACE: usize = 8 + 32 + 2 + 4;
}

#[account]
pub struct Listing {
    pub lister: Pubkey,
    pub nft: Pubkey,
    pub collection: Pubkey,
    pub price: u64,
}

impl Space for Listing {
    const INIT_SPACE: usize = 8 + 32 + 32 + 32 + 8;
}

#[account]
pub struct BidState {
    pub bidder: Pubkey,
    pub price: u64,
}

impl Space for BidState {
    const INIT_SPACE: usize = 8 + 32 + 8;
}
