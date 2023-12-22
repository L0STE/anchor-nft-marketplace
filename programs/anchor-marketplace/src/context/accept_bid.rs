pub use anchor_lang::{
    prelude::*,
    system_program::{Transfer, transfer}
};

use mpl_token_metadata::types::{TransferArgs, UnlockArgs};
pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;


use anchor_spl::{
    token::{Mint, TokenAccount, CloseAccount, close_account}, 
    metadata::{Metadata, MetadataAccount, MasterEditionAccount,
    mpl_token_metadata::instructions::{TransferCpi, TransferCpiAccounts, TransferInstructionArgs, UnlockCpi, UnlockCpiAccounts, UnlockInstructionArgs}}, 
    associated_token::AssociatedToken
};
pub use anchor_spl::token::Token;

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct AcceptBid<'info> {
    #[account(mut)]
    pub lister: Signer<'info>,
    /// CHECK: no need to check it out
    pub bidder: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = lister,
        associated_token::mint = nft,
        associated_token::authority = bidder,
    )]
    pub bidder_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = nft,
        associated_token::authority = lister,
    )]
    pub lister_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes(), marketplace.admin.key().as_ref()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,
    #[account(
        mut, 
        close = lister,
        seeds = [b"listing", marketplace.key().as_ref()],
        bump,
        has_one = lister,
        has_one = nft,
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
        seeds = [b"bidding_vault", bid.key().as_ref()],
        bump,
    )]
    pub bid_vault: SystemAccount<'info>,

    #[account(mut)]
    pub nft: Account<'info, Mint>,
    #[account(mut)]
    pub metadata: Account<'info, MetadataAccount>,
    pub edition: Account<'info, MasterEditionAccount>,

    #[account(address = INSTRUCTIONS_ID)]
    /// CHECK: no need to check it out
    pub sysvar_instruction: AccountInfo<'info>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> AcceptBid<'info> {
    pub fn accept_bid(
        &mut self,
        bumps: AcceptBidBumps
    ) -> Result<()> {

        let bid_key = self.bid.key();
        let seed = &[
            b"bidding_vault",
            bid_key.as_ref(),
            &[bumps.bid]
        ];
        let signer_seeds = &[&seed[..]];

        // Close the listing vault (ATA) > We get the amount sent directly to the lister since it's Solana
        let close_program = self.token_program.to_account_info();
        let close_accounts = CloseAccount {
            account: self.bid_vault.to_account_info(),
            destination: self.lister.to_account_info(),
            authority: self.bid.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(close_program, close_accounts, signer_seeds);

        close_account(cpi_ctx)?;
        
        // Unlock the NFT before transfering it
        let unlock_program = &self.token_program.to_account_info();
        let authority = &self.listing.to_account_info();
        let token_owner = &self.lister.to_account_info();
        let token = &self.lister_ata.to_account_info();
        let mint = &self.nft.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let edition = &self.edition.to_account_info();
        let payer = &self.lister.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instruction.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();

        let unlock_cpi = UnlockCpi::new(
            unlock_program,
            UnlockCpiAccounts {
                authority,
                token_owner: Some(token_owner),
                token,
                mint,
                metadata,
                edition: Some(edition),
                token_record: None,
                payer,
                system_program,
                sysvar_instructions,
                spl_token_program: Some(spl_token_program),
                authorization_rules_program: None,
                authorization_rules: None,
            },
            UnlockInstructionArgs {
                unlock_args: UnlockArgs::V1 {
                    authorization_data: None,
                },
            }
        );

        let marketplace_key = self.marketplace.key();
        let seed = &[
            b"listing",
            marketplace_key.as_ref(),
            &[bumps.listing]
        ];
        let signer_seeds = &[&seed[..]];

        unlock_cpi.invoke_signed(signer_seeds)?;
        
        // Transfer the NFT > Then we close the account of the delegation so we don't need to revoke that.
        let transfer_program = self.token_program.to_account_info();
        let token = &self.lister_ata.to_account_info();
        let token_owner = &self.lister.to_account_info();
        let destination_token = &self.bidder_ata.to_account_info();
        let destination_owner: &AccountInfo<'_> = &self.bidder.to_account_info();
        let mint = &self.nft.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let edition = &self.edition.to_account_info();
        let authority = &self.listing.to_account_info();
        let payer = &self.lister.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instruction.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();
        let spl_ata_program = &self.associated_token_program.to_account_info();        
        
        let transfer_cpi = TransferCpi::new(
            &transfer_program,
            TransferCpiAccounts {
                token,
                token_owner,
                destination_token,
                destination_owner,
                mint,
                metadata,
                edition: Some(edition),
                token_record: None,
                destination_token_record: None,
                authority,
                payer,
                system_program,
                sysvar_instructions,
                spl_token_program,
                spl_ata_program,
                authorization_rules_program: None,
                authorization_rules: None,
            },
            TransferInstructionArgs {
                transfer_args: TransferArgs::V1 {
                    amount: 1,
                    authorization_data: None,
                },
            }
        );

        transfer_cpi.invoke_signed(signer_seeds)?;
        
        Ok(())
    }
}