pub use anchor_lang::{
    prelude::*,
    system_program::{Transfer, transfer}
};

pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;

use anchor_spl::{
    token::{Mint, TokenAccount, Token, CloseAccount, close_account}, 
    metadata::{Metadata, MetadataAccount, MasterEditionAccount,
    mpl_token_metadata::instructions::{TransferCpi, TransferCpiAccounts, TransferInstructionArgs}}, 
    associated_token::AssociatedToken
};

use mpl_token_metadata::types::TransferArgs;

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, address = listing.lister.key())]
    pub lister: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = lister,
        associated_token::mint = nft,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes(), marketplace.admin.key().as_ref()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,
    #[account(
        seeds = [b"fee_vault", marketplace.key().as_ref()],
        bump,
    )]
    pub fee_vault: SystemAccount<'info>,
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
        init_if_needed,
        payer = lister,
        associated_token::mint = nft,
        associated_token::authority = listing,
    )]
    pub listing_vault: Option<Account<'info, TokenAccount>>,

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

impl<'info> Buy<'info> {
    pub fn pay_nft(
        &mut self,
    ) -> Result<()> {

        // Pay for the NFT
        let transfer_program = self.system_program.to_account_info();
        let transfer_accounts = Transfer {
            from: self.buyer.to_account_info(),
            to: self.lister.to_account_info(),
        };
        let transfer_cpi = CpiContext::new(transfer_program, transfer_accounts);

        transfer(transfer_cpi, self.listing.price)?;

        // Pay the Fee
        let transfer_program = self.system_program.to_account_info();
        let transfer_accounts = Transfer {
            from: self.buyer.to_account_info(),
            to: self.fee_vault.to_account_info(),
        };
        let transfer_cpi = CpiContext::new(transfer_program, transfer_accounts);
        
        transfer(transfer_cpi, self.listing.price.checked_mul(self.marketplace.fee as u64).unwrap())?;

        // Pay Royalties (???)
    
        Ok(())
    }

    pub fn send_nft(
        &mut self,
        bumps: BuyBumps,
    ) -> Result<()> {

        let transfer_program = self.token_program.to_account_info();
        let token = &self.listing_vault.as_mut().unwrap().to_account_info();
        let token_owner = &self.listing.to_account_info();
        let destination_token = &self.buyer_ata.to_account_info();
        let destination_owner = &self.buyer.to_account_info();
        let mint = &self.nft.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let edition = &self.edition.to_account_info();
        let authority = &self.listing.to_account_info();
        let payer = &self.buyer.to_account_info();
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

        let marketplace_key = self.marketplace.key();
        let seed = &[
            b"listing",
            marketplace_key.as_ref(),
            &[bumps.listing]
        ];
        let signer_seeds = &[&seed[..]];

        transfer_cpi.invoke_signed(signer_seeds)?;

        // Close the listing vault (ATA)
        let close_program = self.token_program.to_account_info();
        let close_accounts = CloseAccount {
            account: self.listing_vault.as_mut().unwrap().to_account_info(),
            destination: self.lister.to_account_info(),
            authority: self.listing.to_account_info(),
        };
        let close_cpi = CpiContext::new_with_signer(close_program, close_accounts, signer_seeds);

        close_account(close_cpi)?;


        Ok(())
    }
}