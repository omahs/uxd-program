use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use anchor_spl::token::Transfer;

use crate::AccountingEvent;
use crate::Controller;
use crate::ErrorCode;
use crate::CONTROLLER_NAMESPACE;
use crate::MANGO_DEPOSITORY_NAMESPACE;
use crate::MANGO_ACCOUNT_NAMESPACE;
use crate::INSURANCE_PASSTHROUGH_NAMESPACE;
use crate::MangoDepository;
use crate::mango_program;

#[derive(Accounts)]
pub struct DepositInsuranceToMangoDepository<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [CONTROLLER_NAMESPACE], 
        bump = controller.bump,
        has_one = authority @ErrorCode::InvalidAuthority,
    )]
    pub controller: Box<Account<'info, Controller>>,
    #[account(
        mut,
        seeds = [MANGO_DEPOSITORY_NAMESPACE, collateral_mint.key().as_ref()],
        bump = depository.bump,
        has_one = controller @ErrorCode::InvalidController,
        constraint = controller.registered_mango_depositories.contains(&depository.key()) @ErrorCode::InvalidDepository
    )]
    pub depository: Box<Account<'info, MangoDepository>>,
    #[account(
        constraint = collateral_mint.key() == depository.collateral_mint @ErrorCode::InvalidCollateralMint
    )]
    pub collateral_mint: Box<Account<'info, Mint>>,
    #[account(
        constraint = insurance_mint.key() == depository.insurance_mint @ErrorCode::InvalidInsuranceMint
    )]
    pub insurance_mint: Box<Account<'info, Mint>>,
    // The account that contains the funds to deposit
    #[account(
        mut,
        constraint = authority_insurance.mint == depository.insurance_mint @ErrorCode::InvalidAuthorityInsuranceATAMint
    )]
    pub authority_insurance: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [INSURANCE_PASSTHROUGH_NAMESPACE, collateral_mint.key().as_ref(), insurance_mint.key().as_ref()],
        bump = depository.insurance_passthrough_bump,
        constraint = depository.insurance_passthrough == depository_insurance_passthrough_account.key() @ErrorCode::InvalidInsurancePassthroughAccount,
        constraint = depository_insurance_passthrough_account.mint == insurance_mint.key() @ErrorCode::InvalidInsurancePassthroughATAMint,
    )]
    pub depository_insurance_passthrough_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [MANGO_ACCOUNT_NAMESPACE, collateral_mint.key().as_ref()],
        bump = depository.mango_account_bump,
        constraint = depository.mango_account == depository_mango_account.key() @ErrorCode::InvalidMangoAccount,
    )]
    pub depository_mango_account: AccountInfo<'info>,
    // Mango related accounts -------------------------------------------------
    // XXX All these account should be properly constrained
    pub mango_group: AccountInfo<'info>,
    pub mango_cache: AccountInfo<'info>,
    pub mango_root_bank: AccountInfo<'info>,
    #[account(mut)]
    pub mango_node_bank: AccountInfo<'info>,
    #[account(mut)]
    pub mango_vault: Account<'info, TokenAccount>,
    // ------------------------------------------------------------------------
    // programs
    pub token_program: Program<'info, Token>,
    pub mango_program: Program<'info, mango_program::Mango>,
}

pub fn handler(
    ctx: Context<DepositInsuranceToMangoDepository>,
    insurance_amount: u64, // In Mative Units
) -> ProgramResult {
    let collateral_mint = ctx.accounts.collateral_mint.key();

    let depository_signer_seeds: &[&[&[u8]]] = &[&[
        MANGO_DEPOSITORY_NAMESPACE,
        collateral_mint.as_ref(),
        &[ctx.accounts.depository.bump],
    ]];

    // - 1 [TRANSFER INSURANCE TO MANGO] --------------------------------------

    // - Transfers insurance to the passthrough account
    token::transfer(
        ctx.accounts
            .into_transfer_to_passthrough_context(),
        insurance_amount,
    )?;

    // - Deposit Insurance to Mango Account
    mango_program::deposit(
        ctx.accounts
            .into_deposit_to_mango_context()
            .with_signer(depository_signer_seeds),
        insurance_amount,
    )?;

    // - 2 [UPDATE ACCOUNTING] ------------------------------------------------
    ctx.accounts.update_accounting(insurance_amount)?;

    Ok(())
}

impl<'info> DepositInsuranceToMangoDepository<'info> {
    pub fn into_transfer_to_passthrough_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.authority_insurance.to_account_info(),
            to: self
                .depository_insurance_passthrough_account
                .to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    pub fn into_deposit_to_mango_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, mango_program::Deposit<'info>> {
        let cpi_accounts = mango_program::Deposit {
            mango_group: self.mango_group.to_account_info(),
            mango_account: self.depository_mango_account.to_account_info(),
            owner: self.depository.to_account_info(),
            mango_cache: self.mango_cache.to_account_info(),
            mango_root_bank: self.mango_root_bank.to_account_info(),
            mango_node_bank: self.mango_node_bank.to_account_info(),
            mango_vault: self.mango_vault.to_account_info(),
            token_program: self.token_program.to_account_info(),
            owner_token_account: self
                .depository_insurance_passthrough_account
                .to_account_info(),
        };
        let cpi_program = self.mango_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

// Additional convenience methods related to the inputed accounts
impl<'info> DepositInsuranceToMangoDepository<'info> {
    fn update_accounting(
        &mut self,
        insurance_delta: u64,
    ) -> ProgramResult {
        // Mango Depository
        self.depository
            .update_insurance_amount_deposited(AccountingEvent::Deposit, insurance_delta);
        Ok(())
    }
}