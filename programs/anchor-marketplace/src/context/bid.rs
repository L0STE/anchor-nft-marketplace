pub use anchor_lang::{
    prelude::*,
    system_program::{Transfer, transfer}
};

pub use anchor_spl::token::Token;

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct Bid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes(), marketplace.admin.key().as_ref()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,
    #[account(
        seeds = [b"listing", marketplace.key().as_ref()],
        bump,
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        init,
        payer = bidder,
        seeds = [b"bid", listing.key().as_ref(), bidder.key().as_ref()],
        bump,
        space = BidState::INIT_SPACE,
    )]
    pub bid: Account<'info, BidState>,

    #[account(
        seeds = [b"listing_vault", bid.key().as_ref()],
        bump,
    )]
    pub bid_vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Bid<'info> {
    pub fn bid(
        &mut self,
        amount: u64,
    ) -> Result<()> {

        self.bid.set_inner(
            BidState {
                bidder: self.bidder.key(),
                price: amount,
            }
        );

        let transfer_program = self.system_program.to_account_info();
        let transfer_account = Transfer {
            from: self.bidder.to_account_info(),
            to: self.bid_vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(transfer_program, transfer_account);

        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}