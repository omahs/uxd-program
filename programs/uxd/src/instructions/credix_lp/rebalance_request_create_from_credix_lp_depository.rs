use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use anchor_spl::token::TokenAccount;

use crate::error::UxdError;
use crate::events::RebalanceRequestCreateFromCredixLpDepositoryEvent;
use crate::state::controller::Controller;
use crate::state::credix_lp_depository::CredixLpDepository;
use crate::state::identity_depository::IdentityDepository;
use crate::state::mercurial_vault_depository::MercurialVaultDepository;
use crate::utils::compute_value_for_shares_amount_floor;
use crate::validate_is_program_frozen;
use crate::CONTROLLER_NAMESPACE;
use crate::CREDIX_LP_DEPOSITORY_NAMESPACE;
use crate::CREDIX_LP_EXTERNAL_PASS_NAMESPACE;
use crate::CREDIX_LP_EXTERNAL_WITHDRAW_EPOCH_NAMESPACE;
use crate::IDENTITY_DEPOSITORY_NAMESPACE;
use crate::MERCURIAL_VAULT_DEPOSITORY_NAMESPACE;

#[derive(Accounts)]
pub struct RebalanceRequestCreateFromCredixLpDepository<'info> {
    /// #1
    /// Permissionless IX that can be called by anyone at any time
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #2
    #[account(
        mut,
        seeds = [CONTROLLER_NAMESPACE],
        bump = controller.load()?.bump,
        constraint = controller.load()?.identity_depository == identity_depository.key() @UxdError::InvalidDepository,
        constraint = controller.load()?.mercurial_vault_depository == mercurial_vault_depository.key() @UxdError::InvalidDepository,
        constraint = controller.load()?.credix_lp_depository == depository.key() @UxdError::InvalidDepository,
    )]
    pub controller: AccountLoader<'info, Controller>,

    /// #3
    #[account(
        mut,
        seeds = [IDENTITY_DEPOSITORY_NAMESPACE],
        bump = identity_depository.load()?.bump,
    )]
    pub identity_depository: AccountLoader<'info, IdentityDepository>,

    /// #4
    #[account(
        mut,
        seeds = [MERCURIAL_VAULT_DEPOSITORY_NAMESPACE, mercurial_vault_depository.load()?.mercurial_vault.key().as_ref(), mercurial_vault_depository.load()?.collateral_mint.as_ref()],
        bump = mercurial_vault_depository.load()?.bump,
        has_one = controller @UxdError::InvalidController,
        has_one = collateral_mint @UxdError::InvalidCollateralMint,
    )]
    pub mercurial_vault_depository: AccountLoader<'info, MercurialVaultDepository>,

    /// #5
    #[account(
        mut,
        seeds = [
            CREDIX_LP_DEPOSITORY_NAMESPACE,
            depository.load()?.credix_global_market_state.key().as_ref(),
            depository.load()?.collateral_mint.as_ref()
        ],
        bump = depository.load()?.bump,
        has_one = controller @UxdError::InvalidController,
        has_one = collateral_mint @UxdError::InvalidCollateralMint,
        has_one = depository_shares @UxdError::InvalidDepositoryShares,
        has_one = credix_global_market_state @UxdError::InvalidCredixGlobalMarketState,
        has_one = credix_signing_authority @UxdError::InvalidCredixSigningAuthority,
        has_one = credix_liquidity_collateral @UxdError::InvalidCredixLiquidityCollateral,
        has_one = credix_shares_mint @UxdError::InvalidCredixSharesMint,
    )]
    pub depository: AccountLoader<'info, CredixLpDepository>,

    /// #6
    pub collateral_mint: Box<Account<'info, Mint>>,

    /// #7
    #[account(mut)]
    pub depository_shares: Box<Account<'info, TokenAccount>>,

    /// #8
    pub credix_global_market_state: Box<Account<'info, credix_client::GlobalMarketState>>,

    /// #9
    /// CHECK: unused by us, checked by credix
    pub credix_signing_authority: AccountInfo<'info>,

    /// #10
    #[account(mut)]
    pub credix_liquidity_collateral: Box<Account<'info, TokenAccount>>,

    /// #11
    #[account(mut)]
    pub credix_shares_mint: Box<Account<'info, Mint>>,

    /// #12
    #[account(
        owner = credix_client::ID,
        seeds = [
            credix_global_market_state.key().as_ref(),
            depository.key().as_ref(),
            CREDIX_LP_EXTERNAL_PASS_NAMESPACE
        ],
        bump,
        seeds::program = credix_client::ID,
        constraint = credix_pass.user == depository.key() @UxdError::InvalidCredixPass,
        constraint = credix_pass.disable_withdrawal_fee @UxdError::InvalidCredixPassNoFees,
    )]
    pub credix_pass: Account<'info, credix_client::CredixPass>,

    /// #13
    #[account(
        mut,
        owner = credix_client::ID,
        seeds = [
            credix_global_market_state.key().as_ref(),
            &credix_global_market_state.latest_withdraw_epoch_idx.to_le_bytes(),
            CREDIX_LP_EXTERNAL_WITHDRAW_EPOCH_NAMESPACE
        ],
        bump,
        seeds::program = credix_client::ID,
    )]
    pub credix_withdraw_epoch: Account<'info, credix_client::WithdrawEpoch>,

    /// #14
    /// CHECK: initialized by credix program, not manipulated by us
    #[account(mut)]
    pub credix_withdraw_request: AccountInfo<'info>,

    /// #15
    pub system_program: Program<'info, System>,
    /// #16
    pub credix_program: Program<'info, credix_client::program::Credix>,
}

pub(crate) fn handler(ctx: Context<RebalanceRequestCreateFromCredixLpDepository>) -> Result<()> {
    let credix_global_market_state = ctx.accounts.depository.load()?.credix_global_market_state;
    let collateral_mint = ctx.accounts.depository.load()?.collateral_mint;
    let depository_pda_signer: &[&[&[u8]]] = &[&[
        CREDIX_LP_DEPOSITORY_NAMESPACE,
        credix_global_market_state.as_ref(),
        collateral_mint.as_ref(),
        &[ctx.accounts.depository.load()?.bump],
    ]];

    // ---------------------------------------------------------------------
    // -- Phase 1
    // -- Check if the withdraw request period is active at the moment
    // ---------------------------------------------------------------------

    {
        let start_of_request_phase_timestamp = ctx.accounts.credix_withdraw_epoch.go_live;
        let end_of_request_phase_timestamp = start_of_request_phase_timestamp
            .checked_add(ctx.accounts.credix_withdraw_epoch.request_seconds.into())
            .ok_or(UxdError::MathError)?;
        require!(
            (start_of_request_phase_timestamp..end_of_request_phase_timestamp)
                .contains(&Clock::get()?.unix_timestamp),
            UxdError::InvalidCredixWithdrawEpochRequestPeriod,
        );
    }

    // ---------------------------------------------------------------------
    // -- Phase 2
    // -- Calculate the amount we "may" want to withdraw
    // -- Just in case, we WAY overshoot the requested amount,
    // -- so that the execution can choose an exact amount without worring we didnt request enough
    // ---------------------------------------------------------------------

    let requested_collateral_amount = {
        let liquidity_collateral_amount: u64 = ctx.accounts.credix_liquidity_collateral.amount;
        let outstanding_collateral_amount: u64 = ctx
            .accounts
            .credix_global_market_state
            .pool_outstanding_credit;
        let total_shares_supply: u64 = ctx.accounts.credix_shares_mint.supply;
        let total_shares_value: u64 = liquidity_collateral_amount
            .checked_add(outstanding_collateral_amount)
            .ok_or(UxdError::MathError)?;
        let owned_shares_amount: u64 = ctx.accounts.depository_shares.amount;
        compute_value_for_shares_amount_floor(
            owned_shares_amount,
            total_shares_supply,
            total_shares_value,
        )?
    };

    // ---------------------------------------------------------------------
    // -- Phase 3
    // -- We just create the credix withdraw request with the computed withdrawal amount
    // ---------------------------------------------------------------------

    msg!(
        "[rebalance_request_create_from_credix_lp_depository:requested_collateral_amount:{}]",
        requested_collateral_amount
    );
    credix_client::cpi::create_withdraw_request(
        ctx.accounts
            .into_create_withdraw_request_from_credix_lp_context()
            .with_signer(depository_pda_signer),
        requested_collateral_amount,
    )?;

    // ---------------------------------------------------------------------
    // -- Phase 4
    // -- Emit resulting event
    // ---------------------------------------------------------------------

    emit!(RebalanceRequestCreateFromCredixLpDepositoryEvent {
        controller_version: ctx.accounts.controller.load()?.version,
        depository_version: ctx.accounts.depository.load()?.version,
        controller: ctx.accounts.controller.key(),
        depository: ctx.accounts.depository.key(),
        requested_collateral_amount,
    });

    // Done
    Ok(())
}

// Into functions
impl<'info> RebalanceRequestCreateFromCredixLpDepository<'info> {
    pub fn into_create_withdraw_request_from_credix_lp_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, credix_client::cpi::accounts::CreateWithdrawRequest<'info>>
    {
        let cpi_accounts = credix_client::cpi::accounts::CreateWithdrawRequest {
            payer: self.payer.to_account_info(),
            investor: self.depository.to_account_info(),
            investor_lp_token_account: self.depository_shares.to_account_info(),
            global_market_state: self.credix_global_market_state.to_account_info(),
            signing_authority: self.credix_signing_authority.to_account_info(),
            liquidity_pool_token_account: self.credix_liquidity_collateral.to_account_info(),
            lp_token_mint: self.credix_shares_mint.to_account_info(),
            credix_pass: self.credix_pass.to_account_info(),
            withdraw_epoch: self.credix_withdraw_epoch.to_account_info(),
            withdraw_request: self.credix_withdraw_request.to_account_info(),
            system_program: self.system_program.to_account_info(),
        };
        let cpi_program = self.credix_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

// Validate
impl<'info> RebalanceRequestCreateFromCredixLpDepository<'info> {
    pub(crate) fn validate(&self) -> Result<()> {
        validate_is_program_frozen(self.controller.load()?)?;
        Ok(())
    }
}
