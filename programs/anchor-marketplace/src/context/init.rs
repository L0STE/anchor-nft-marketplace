pub use anchor_lang::prelude::*;

pub use anchor_spl::token::Token;

pub use crate::state::*;

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [b"marketplace", name.as_str().as_bytes(), admin.key().as_ref()],
        bump,
        space = Marketplace::INIT_SPACE + name.len(),
    )]
    pub marketplace: Account<'info, Marketplace>,
    #[account(
        seeds = [b"fee_vault", marketplace.key().as_ref()],
        bump,
    )]
    pub fee_vault: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info>{
    pub fn initialize(
        &mut self,
        name: String,
        fee: u16,
    ) -> Result<()> {
        self.marketplace.set_inner(
            Marketplace {
                admin: self.admin.key(),
                fee,
                name,
            }
        );

        Ok(())
    }
}