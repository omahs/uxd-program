use crate::error::UxdError;
use crate::Controller;
use crate::MangoDepository;
use crate::CONTROLLER_NAMESPACE;
use crate::MANGO_DEPOSITORY_NAMESPACE;
use anchor_lang::prelude::*;

/// Takes 3 accounts
#[derive(Accounts)]
pub struct DisableDepositoryMinting<'info> {
    /// #1 Authored call accessible only to the signer matching Controller.authority
    pub authority: Signer<'info>,

    /// #2 The top level UXDProgram on chain account managing the redeemable mint
    #[account(
        seeds = [CONTROLLER_NAMESPACE],
        bump = controller.load()?.bump,
        has_one = authority @UxdError::InvalidAuthority,
        constraint = controller.load()?.registered_mango_depositories.contains(&depository.key()) @UxdError::InvalidDepository
    )]
    pub controller: AccountLoader<'info, Controller>,

    /// #3 UXDProgram on chain account bound to a Controller instance
    /// The `MangoDepository` manages a MangoAccount for a single Collateral
    #[account(
        mut,
        seeds = [MANGO_DEPOSITORY_NAMESPACE, depository.load()?.collateral_mint.as_ref()],
        bump = depository.load()?.bump,
        has_one = controller @UxdError::InvalidController,
    )]
    pub depository: AccountLoader<'info, MangoDepository>,
}

pub fn handler(ctx: Context<DisableDepositoryMinting>, disable: bool) -> Result<()> {
    let depository = &mut ctx.accounts.depository.load_mut()?;
    depository.minting_disabled = disable;
    Ok(())
}

// Validate input arguments
impl<'info> DisableDepositoryMinting<'info> {
    pub fn validate(&self, disable_minting: bool) -> Result<()> {
        require!(
            self.depository.load()?.minting_disabled != disable_minting,
            UxdError::MintingAlreadyDisabledOrEnabled
        );
        Ok(())
    }
}