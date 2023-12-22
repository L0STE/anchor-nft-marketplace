pub use anchor_lang::{
    prelude::*,
    system_program::{Transfer, transfer}
};

pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;

use anchor_spl::{
    token::{Mint, TokenAccount, Token}, 
    metadata::{Metadata, MetadataAccount, MasterEditionAccount,
    mpl_token_metadata::instructions::{TransferCpi, TransferCpiAccounts, TransferInstructionArgs, UnlockCpi, UnlockCpiAccounts, UnlockInstructionArgs}}, 
    associated_token::AssociatedToken
};

use mpl_token_metadata::types::{TransferArgs, UnlockArgs, Creator};
use solana_program::sysvar::instructions::{load_current_index_checked, load_instruction_at_checked};

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
        payer = buyer,
        associated_token::mint = nft,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = nft,
        associated_token::authority = lister,
    )]
    pub lister_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes(), marketplace.admin.key().as_ref()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,
    #[account(
        mut,
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
    pub fn buy(
        &mut self,
        bumps: BuyBumps,
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
        
        transfer(transfer_cpi, (self.listing.price.checked_mul(self.marketplace.fee as u64).unwrap()).checked_div(10000).unwrap())?;

        // Make sure that we pay Royalties
        if self.metadata.seller_fee_basis_points != 0 && self.metadata.creators.is_some() {
            let seller_fee_basis_points = self.metadata.seller_fee_basis_points;
            let amount_to_split = self.listing.price
                .checked_mul(seller_fee_basis_points as u64)
                .unwrap()
                .checked_div(10000)
                .unwrap();
            let creators = self.metadata.creators.as_ref().unwrap()
                .iter()
                .filter(|creator| creator.share > 0)
                .collect::<Vec<&Creator>>();
        
            let index = load_current_index_checked(&self.sysvar_instruction.to_account_info())?;

            for (i, _creator) in creators.iter().enumerate() {
                let ix = load_instruction_at_checked(index as usize + 1 + i, &self.sysvar_instruction.to_account_info())?;
                
                let creator_amount = amount_to_split
                    .checked_mul(_creator.share as u64)
                    .unwrap()
                    .checked_div(100)
                    .unwrap();
                
                require_keys_eq!(ix.program_id, self.system_program.key(), InstrospectionError::InvalidTokenProgram);
                require_eq!(ix.data[0], 2u8, InstrospectionError::InvalidIx);
                require!(ix.data[4..12].eq(&creator_amount.to_le_bytes()), InstrospectionError::InvalidAmount);
                require_keys_eq!(ix.accounts.get(1).unwrap().pubkey, _creator.address, InstrospectionError::InvalidCreator);
            }
        }
    
        // Unlock the NFT before transfering it
        let unlock_program = &self.token_program.to_account_info();
        let authority = &self.listing.to_account_info();
        let token_owner = &self.lister.to_account_info();
        let token = &self.lister_ata.to_account_info();
        let mint = &self.nft.to_account_info();
        let metadata = &self.metadata.to_account_info();
        let edition = &self.edition.to_account_info();
        let payer = &self.buyer.to_account_info();
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

        transfer_cpi.invoke_signed(signer_seeds)?;
    
        Ok(())
    }
}