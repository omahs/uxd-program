use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use mango::state::MangoAccount;
use std::mem::size_of;

use crate::PROGRAM_VERSION;
use crate::mango_program;
use crate::MangoDepository;
use crate::Controller;
use crate::ErrorCode;
use crate::CONTROLLER_NAMESPACE;
use crate::MANGO_DEPOSITORY_NAMESPACE;
use crate::COLLATERAL_PASSTHROUGH_NAMESPACE;
use crate::INSURANCE_PASSTHROUGH_NAMESPACE;
use crate::MANGO_ACCOUNT_NAMESPACE;

const MANGO_ACCOUNT_SPAN: usize = size_of::<MangoAccount>();

#[derive(Accounts)]
#[instruction(
    bump: u8,
    collateral_passthrough_bump: u8,
    insurance_passthrough_bump: u8,
    mango_account_bump: u8,
)]
pub struct RegisterMangoDepository<'info> {
    pub authority: Signer<'info>,
    // In order to use with governance program, as the PDA cannot be the payer in nested TX.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [CONTROLLER_NAMESPACE], 
        bump = controller.bump,
        has_one = authority @ErrorCode::InvalidAuthority,
    )]
    pub controller: Box<Account<'info, Controller>>,
    #[account(
        init,
        seeds = [MANGO_DEPOSITORY_NAMESPACE, collateral_mint.key().as_ref()],
        bump = bump,
        payer = payer,
    )]
    pub depository: Box<Account<'info, MangoDepository>>,
    pub collateral_mint: Box<Account<'info, Mint>>,
    pub insurance_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        seeds = [COLLATERAL_PASSTHROUGH_NAMESPACE, collateral_mint.key().as_ref()],
        bump = collateral_passthrough_bump,
        token::mint = collateral_mint,
        token::authority = depository,
        payer = payer,
    )]
    pub depository_collateral_passthrough_account: Account<'info, TokenAccount>,
    #[account(
        init,
        seeds = [INSURANCE_PASSTHROUGH_NAMESPACE, collateral_mint.key().as_ref(), insurance_mint.key().as_ref()],
        bump = insurance_passthrough_bump,
        token::mint = insurance_mint,
        token::authority = depository,
        payer = payer,
    )]
    pub depository_insurance_passthrough_account: Account<'info, TokenAccount>,
    #[account(
        init,
        seeds = [MANGO_ACCOUNT_NAMESPACE, collateral_mint.key().as_ref()],
        bump = mango_account_bump,
        owner = mango_program::Mango::id(),
        payer = payer,
        space = MANGO_ACCOUNT_SPAN,
    )]
    pub depository_mango_account: AccountInfo<'info>,
    // Mango CPI
    pub mango_group: AccountInfo<'info>,
    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub mango_program: Program<'info, mango_program::Mango>,
    // sysvar
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<RegisterMangoDepository>,
    bump: u8, 
    collateral_passthrough_bump: u8,
    insurance_passthrough_bump: u8,
    mango_account_bump: u8
) -> ProgramResult {
    let collateral_mint = ctx.accounts.collateral_mint.key();
    let insurance_mint = ctx.accounts.insurance_mint.key();

    // - Initialize Mango Account
    let depository_signer_seed: &[&[&[u8]]] = &[&[
        MANGO_DEPOSITORY_NAMESPACE,
        collateral_mint.as_ref(),
        &[bump],
    ]];
    mango_program::initialize_mango_account(
        ctx.accounts
            .into_mango_account_initialization_context()
            .with_signer(depository_signer_seed),
    )?;

    // - Initialize Depository state
    ctx.accounts.depository.bump = bump;
    ctx.accounts.depository.collateral_passthrough_bump = collateral_passthrough_bump;
    ctx.accounts.depository.insurance_passthrough_bump = insurance_passthrough_bump;
    ctx.accounts.depository.mango_account_bump = mango_account_bump;
    ctx.accounts.depository.version = PROGRAM_VERSION;
    ctx.accounts.depository.collateral_mint = collateral_mint;
    ctx.accounts.depository.collateral_passthrough = ctx.accounts.depository_collateral_passthrough_account.key();
    ctx.accounts.depository.insurance_mint = insurance_mint;
    ctx.accounts.depository.insurance_passthrough = ctx.accounts.depository_insurance_passthrough_account.key();
    ctx.accounts.depository.mango_account = ctx.accounts.depository_mango_account.key();
    ctx.accounts.depository.controller = ctx.accounts.controller.key();
    ctx.accounts.depository.insurance_amount_deposited = u128::MIN;
    ctx.accounts.depository.collateral_amount_deposited = u128::MIN;
    ctx.accounts.depository.redeemable_amount_under_management = u128::MIN;

    // - Update Controller state
    ctx.accounts.add_new_registered_mango_depository_entry_to_controller()?;

    Ok(())
}

impl<'info> RegisterMangoDepository<'info> {
    pub fn into_mango_account_initialization_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, mango_program::InitMangoAccount<'info>> {
        let cpi_accounts = mango_program::InitMangoAccount {
            mango_group: self.mango_group.to_account_info(),
            mango_account: self.depository_mango_account.to_account_info(),
            owner: self.depository.to_account_info(),
            rent: self.rent.to_account_info(),
        };
        let cpi_program = self.mango_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}


impl<'info> RegisterMangoDepository<'info> {
    pub fn add_new_registered_mango_depository_entry_to_controller(
        &mut self,
    ) -> ProgramResult {
        let mango_depository_id = self.depository.key();
        self.controller.add_registered_mango_depository_entry(mango_depository_id)?;
        Ok(())
    }
}