pub use anchor_lang::prelude::*;
pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;

use anchor_spl::{
    token::{Mint, TokenAccount}, 
    metadata::{Metadata, MetadataAccount, MasterEditionAccount,
        mpl_token_metadata::instructions::{UnlockCpi, UnlockCpiAccounts, UnlockInstructionArgs, RevokeCpi, RevokeCpiAccounts, RevokeInstructionArgs}, 
    },
    associated_token::AssociatedToken,
};
pub use anchor_spl::token::Token;
use mpl_token_metadata::types::{RevokeArgs, UnlockArgs};

pub use crate::state::*;
pub use crate::errors::*;

#[derive(Accounts)]
pub struct Delist<'info> {
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
        mut,
        close = lister,
        seeds = [b"listing", marketplace.key().as_ref()],
        bump,
        has_one = lister,
        has_one = nft,
    )]
    pub listing: Account<'info, Listing>,

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

impl<'info> Delist<'info> {
    pub fn delist(
        &mut self,
        bumps: DelistBumps,
    ) -> Result<()> {

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
    
        let revoke_program = &self.token_metadata_program.to_account_info();
        let delegate = &self.listing.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let token = &self.lister_ata.to_account_info();
        let authority = &self.lister.to_account_info();
        let payer = &self.lister.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instruction.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();

        let revoke_cpi = RevokeCpi::new(
            revoke_program,
            RevokeCpiAccounts {
                delegate_record: None,
                delegate,
                metadata,
                master_edition: Some(edition),
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
            RevokeInstructionArgs {
                revoke_args: RevokeArgs::StandardV1 
            },
        );

        revoke_cpi.invoke()?;

        Ok(())
    }

}