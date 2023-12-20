pub use anchor_lang::{
    prelude::*,
    system_program::{Transfer, transfer}
};
pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_spl::token::{ Token, CloseAccount, close_account};

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct CancelBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes(), marketplace.admin.key().as_ref()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,
    #[account(
        mut, 
        seeds = [b"listing", marketplace.key().as_ref()],
        bump,
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        mut,
        close = bidder,
        seeds = [b"bid", listing.key().as_ref(), bidder.key().as_ref()],
        bump,
        has_one = bidder,
    )]
    pub bid: Account<'info, BidState>,
    #[account(
        seeds = [b"listing_vault", bid.key().as_ref()],
        bump,
    )]
    pub bid_vault: SystemAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> CancelBid<'info> {
    pub fn cancel_bid(
        &mut self,
        bumps: CancelBidBumps
    ) -> Result<()> {

        let bid_key = self.bid.key();
        let seed = &[
            b"listing_vault",
            bid_key.as_ref(),
            &[bumps.bid]
        ];
        let signer_seeds = &[&seed[..]];

        // Close the listing vault (ATA) == return the bid amount to the bidder
        let close_program = self.token_program.to_account_info();
        let close_accounts = CloseAccount {
            account: self.bid_vault.to_account_info(),
            destination: self.bidder.to_account_info(),
            authority: self.bid.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(close_program, close_accounts, signer_seeds);

        close_account(cpi_ctx)?;
        
        Ok(())
    }
}