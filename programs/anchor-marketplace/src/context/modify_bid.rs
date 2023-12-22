pub use anchor_lang::{
    prelude::*,
    system_program::{Transfer, transfer}
};

use anchor_spl::token::Token;

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct ModifyBid<'info> {
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

impl<'info> ModifyBid<'info> {
    pub fn modify_bid(
        &mut self,
        amount: u64,
        bumps: ModifyBidBumps
    ) -> Result<()> {

        require!(amount > 0 && amount != self.bid.price, MarketplaceError::InvalidAmount);

        if amount > self.bid.price {

            let transfer_program = self.system_program.to_account_info();
            let transfer_accounts = Transfer {
                from: self.bidder.to_account_info(),
                to: self.bid_vault.to_account_info(),
            };
            let transfer_cpi = CpiContext::new(transfer_program, transfer_accounts);

            transfer(transfer_cpi, amount-self.bid.price)?;

            self.bid.price = amount;

        } else {

            let transfer_program = self.system_program.to_account_info();
            let transfer_accounts = Transfer {
                from: self.bidder.to_account_info(),
                to: self.bid_vault.to_account_info(),
            };

            let bid_key = self.bid.key();
            let seed = &[
                b"listing_vault",
                bid_key.as_ref(),
                &[bumps.bid]
            ];
            let signer_seeds = &[&seed[..]];

            let transfer_cpi = CpiContext::new_with_signer(transfer_program, transfer_accounts, signer_seeds);

            transfer(transfer_cpi, self.bid.price-amount)?;

            self.bid.price = amount;
            
        }
        
        Ok(())
    }
}