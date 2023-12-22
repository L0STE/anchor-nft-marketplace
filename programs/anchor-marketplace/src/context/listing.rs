pub use anchor_lang::prelude::*;
pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;

use anchor_spl::{
    token::{Mint, TokenAccount}, 
    metadata::{Metadata, MetadataAccount, MasterEditionAccount,
    mpl_token_metadata::{
        instructions::{DelegateCpi, DelegateCpiAccounts, DelegateInstructionArgs, LockCpi, LockCpiAccounts, LockInstructionArgs},
        types::{TokenStandard, Collection},
    }}, 
    associated_token::AssociatedToken
};
pub use anchor_spl::token::Token;
use mpl_token_metadata::types::{DelegateArgs, LockArgs };

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub lister: Signer<'info>,
    #[account(
        init_if_needed,
        payer = lister,
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
        init,
        payer = lister,
        seeds = [b"listing", marketplace.key().as_ref()],
        bump,
        space = Listing::INIT_SPACE,
    )]
    pub listing: Account<'info, Listing>,

    pub collection: Account<'info, Mint>,
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

impl<'info> List<'info> {
    pub fn list(
        &mut self,
        price: u64,
        bumps: ListBumps,
    ) -> Result<()> {

        require!(self.metadata.token_standard.clone().unwrap() == TokenStandard::NonFungible, MarketplaceError::InvalidTokenStandard);
        require!(self.metadata.collection.clone().unwrap() == Collection{verified: true, key: self.collection.key()}, MarketplaceError::InvalidCollection); 

        self.listing.set_inner(
            Listing {
                lister: self.lister.key(),
                nft: self.nft.key(),
                collection: self.collection.key(),
                price,
            }
        );

        let transfer_program = &self.token_program.to_account_info();
        let delegate = &self.listing.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let master_edition = &self.edition.to_account_info();
        let mint = &self.nft.to_account_info();
        let token = &self.lister_ata.to_account_info();
        let authority = &self.lister.to_account_info();
        let payer = &self.lister.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instruction.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();

        let delegate_cpi = DelegateCpi::new(
            transfer_program,
            DelegateCpiAccounts {
                delegate_record: None,
                delegate,
                metadata,
                master_edition: Some(master_edition),
                token_record: None,
                mint,
                token: Some(token),
                authority,
                payer,
                system_program,
                sysvar_instructions,
                spl_token_program: Some(spl_token_program),
                authorization_rules_program: None,
                authorization_rules: None,
                
            },
            DelegateInstructionArgs {
                delegate_args: DelegateArgs::StandardV1 {
                    amount: 1, 
                },
            },
        );

        delegate_cpi.invoke()?;

        let authority = &self.listing.to_account_info();
        let token_owner = &self.lister.to_account_info();
        
        let lock_cpi = LockCpi::new(
            transfer_program,
            LockCpiAccounts {
                authority,
                token_owner: Some(token_owner),
                token, 
                mint,
                metadata,
                edition: Some(master_edition),
                token_record: None,
                payer,
                system_program,
                sysvar_instructions,
                spl_token_program: Some(spl_token_program),
                authorization_rules_program: None,
                authorization_rules: None,
            },
            LockInstructionArgs {
                lock_args: LockArgs::V1 {
                    authorization_data: None,
                },
            },
        );

        let marketplace_key = self.marketplace.key();
        let seed = &[
            b"listing",
            marketplace_key.as_ref(),
            &[bumps.listing]
        ];
        let signer_seeds = &[&seed[..]];

        lock_cpi.invoke_signed(signer_seeds)?;

        Ok(())
    }
}