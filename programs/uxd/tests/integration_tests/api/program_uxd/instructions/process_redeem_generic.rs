use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_token::state::Account;
use spl_token::state::Mint;

use uxd::state::Controller;
use uxd::state::CredixLpDepository;
use uxd::state::IdentityDepository;
use uxd::state::MercurialVaultDepository;
use uxd::utils::calculate_amount_less_fees;

use crate::integration_tests::api::program_credix;
use crate::integration_tests::api::program_mercurial;
use crate::integration_tests::api::program_test_context;
use crate::integration_tests::api::program_uxd;

pub async fn process_redeem_generic(
    program_test_context: &mut ProgramTestContext,
    payer: &Keypair,
    collateral_mint: &Pubkey,
    mercurial_vault_depository_0_vault_lp_mint: &Pubkey,
    user: &Keypair,
    user_collateral: &Pubkey,
    user_redeemable: &Pubkey,
    redeemable_amount: u64,
    identity_depository_redeemable_amount: u64,
    mercurial_vault_depository_0_redeemable_amount: u64,
) -> Result<(), program_test_context::ProgramTestError> {
    // Find needed accounts
    let controller = program_uxd::accounts::find_controller_pda().0;
    let redeemable_mint = program_uxd::accounts::find_redeemable_mint_pda().0;

    // Find identity depository related accounts
    let identity_depository = program_uxd::accounts::find_identity_depository_pda().0;
    let identity_depository_collateral_vault =
        program_uxd::accounts::find_identity_depository_collateral_vault_pda().0;

    // Find mercurial related accounts
    let mercurial_base_0 = program_mercurial::accounts::find_base();
    let mercurial_vault_depository_0_vault =
        program_mercurial::accounts::find_vault_pda(collateral_mint, &mercurial_base_0.pubkey()).0;
    let mercurial_vault_depository_0 = program_uxd::accounts::find_mercurial_vault_depository_pda(
        collateral_mint,
        &mercurial_vault_depository_0_vault,
    )
    .0;
    let mercurial_vault_depository_0_lp_token_vault =
        program_uxd::accounts::find_mercurial_vault_depository_lp_token_vault_pda(
            collateral_mint,
            &mercurial_vault_depository_0_vault,
        )
        .0;
    let mercurial_vault_depository_0_collateral_token_safe =
        program_mercurial::accounts::find_token_vault_pda(&mercurial_vault_depository_0_vault).0;

    // Find credix related accounts
    let credix_market_seeds_0 = program_credix::accounts::find_market_seeds();
    let credix_lp_depository_0_global_market_state =
        program_credix::accounts::find_global_market_state_pda(&credix_market_seeds_0).0;
    let credix_lp_depository_0 = program_uxd::accounts::find_credix_lp_depository_pda(
        collateral_mint,
        &credix_lp_depository_0_global_market_state,
    )
    .0;

    // Read state before
    let redeemable_mint_before =
        program_test_context::read_account_packed::<Mint>(program_test_context, &redeemable_mint)
            .await?;
    let controller_before =
        program_test_context::read_account_anchor::<Controller>(program_test_context, &controller)
            .await?;

    let identity_depository_before =
        program_test_context::read_account_anchor::<IdentityDepository>(
            program_test_context,
            &identity_depository,
        )
        .await?;
    let mercurial_vault_depository_0_before = program_test_context::read_account_anchor::<
        MercurialVaultDepository,
    >(
        program_test_context, &mercurial_vault_depository_0
    )
    .await?;
    let credix_lp_depository_0_before = program_test_context::read_account_anchor::<
        CredixLpDepository,
    >(program_test_context, &credix_lp_depository_0)
    .await?;

    let user_collateral_amount_before =
        program_test_context::read_account_packed::<Account>(program_test_context, user_collateral)
            .await?
            .amount;
    let user_redeemable_amount_before =
        program_test_context::read_account_packed::<Account>(program_test_context, user_redeemable)
            .await?
            .amount;

    // Execute IX
    let accounts = uxd::accounts::RedeemGeneric {
        payer: payer.pubkey(),
        user: user.pubkey(),
        controller,
        collateral_mint: *collateral_mint,
        redeemable_mint,
        user_collateral: *user_collateral,
        user_redeemable: *user_redeemable,
        identity_depository,
        identity_depository_collateral_vault,
        mercurial_vault_depository_0,
        mercurial_vault_depository_0_vault,
        mercurial_vault_depository_0_vault_lp_mint: *mercurial_vault_depository_0_vault_lp_mint,
        mercurial_vault_depository_0_lp_token_vault,
        mercurial_vault_depository_0_collateral_token_safe,
        credix_lp_depository_0,
        system_program: anchor_lang::system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        mercurial_vault_program: mercurial_vault::ID,
        uxd_program: uxd::ID,
        rent: anchor_lang::solana_program::sysvar::rent::ID,
    };
    let payload = uxd::instruction::RedeemGeneric { redeemable_amount };
    let instruction = Instruction {
        program_id: uxd::id(),
        accounts: accounts.to_account_metas(None),
        data: payload.data(),
    };
    program_test_context::process_instruction_with_signer(
        program_test_context,
        instruction,
        payer,
        user,
    )
    .await?;

    // Read state after
    let redeemable_mint_after =
        program_test_context::read_account_packed::<Mint>(program_test_context, &redeemable_mint)
            .await?;
    let controller_after =
        program_test_context::read_account_anchor::<Controller>(program_test_context, &controller)
            .await?;

    let identity_depository_after =
        program_test_context::read_account_anchor::<IdentityDepository>(
            program_test_context,
            &identity_depository,
        )
        .await?;
    let mercurial_vault_depository_0_after = program_test_context::read_account_anchor::<
        MercurialVaultDepository,
    >(
        program_test_context, &mercurial_vault_depository_0
    )
    .await?;
    let credix_lp_depository_0_after = program_test_context::read_account_anchor::<
        CredixLpDepository,
    >(program_test_context, &credix_lp_depository_0)
    .await?;

    let user_collateral_amount_after =
        program_test_context::read_account_packed::<Account>(program_test_context, user_collateral)
            .await?
            .amount;
    let user_redeemable_amount_after =
        program_test_context::read_account_packed::<Account>(program_test_context, user_redeemable)
            .await?
            .amount;

    // Compute identity_depository amounts
    let identity_depository_collateral_amount = identity_depository_redeemable_amount;

    // Compute mercurial_vault_depository_0 amounts
    let mercurial_vault_depository_0_collateral_amount = calculate_amount_less_fees(
        mercurial_vault_depository_0_redeemable_amount,
        mercurial_vault_depository_0_before.redeeming_fee_in_bps,
    )
    .map_err(program_test_context::ProgramTestError::Anchor)?;
    let mercurial_vault_depository_0_fees_amount = mercurial_vault_depository_0_redeemable_amount
        - mercurial_vault_depository_0_collateral_amount;

    // Compute total amounts
    let total_redeemable_amount =
        identity_depository_redeemable_amount + mercurial_vault_depository_0_redeemable_amount;
    let total_collateral_amount =
        identity_depository_collateral_amount + mercurial_vault_depository_0_collateral_amount;

    // redeemable_mint.supply must have decreased by the redeemed amount (equivalent to redeemable_amount)
    let redeemable_mint_supply_before = redeemable_mint_before.supply;
    let redeemable_mint_supply_after = redeemable_mint_after.supply;
    assert_eq!(
        redeemable_mint_supply_before - total_redeemable_amount,
        redeemable_mint_supply_after,
    );

    // controller.redeemable_circulating_supply must have decreased by the redeemed amount (equivalent to redeemable_amount)
    let redeemable_circulating_supply_before =
        u64::try_from(controller_before.redeemable_circulating_supply).unwrap();
    let redeemable_circulating_supply_after =
        u64::try_from(controller_after.redeemable_circulating_supply).unwrap();
    assert_eq!(
        redeemable_circulating_supply_before - total_redeemable_amount,
        redeemable_circulating_supply_after,
    );

    // identity_depository.redeemable_amount_under_management must have decreased by the redeemed amount (equivalent to redeemable_amount)
    let identity_depository_redeemable_amount_under_management_before =
        u64::try_from(identity_depository_before.redeemable_amount_under_management).unwrap();
    let identity_depository_redeemable_amount_under_management_after =
        u64::try_from(identity_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        identity_depository_redeemable_amount_under_management_before
            - identity_depository_redeemable_amount,
        identity_depository_redeemable_amount_under_management_after,
    );

    // identity_depository.collateral_amount_deposited must have decreased by the withdrawn amount (equivalent to collateral_amount)
    let identity_depository_collateral_amount_deposited_before =
        u64::try_from(identity_depository_before.collateral_amount_deposited).unwrap();
    let identity_depository_collateral_amount_deposited_after =
        u64::try_from(identity_depository_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        identity_depository_collateral_amount_deposited_before
            - identity_depository_collateral_amount,
        identity_depository_collateral_amount_deposited_after,
    );

    // mercurial_vault_depository_0.redeemable_amount_under_management must have decreased by the redeemed amount (equivalent to redeemable_amount)
    let mercurial_vault_depository_0_redeemable_amount_under_management_before =
        u64::try_from(mercurial_vault_depository_0_before.redeemable_amount_under_management)
            .unwrap();
    let mercurial_vault_depository_0_redeemable_amount_under_management_after =
        u64::try_from(mercurial_vault_depository_0_after.redeemable_amount_under_management)
            .unwrap();
    assert_eq!(
        mercurial_vault_depository_0_redeemable_amount_under_management_before
            - mercurial_vault_depository_0_redeemable_amount,
        mercurial_vault_depository_0_redeemable_amount_under_management_after,
    );

    // mercurial_vault_depository_0.redeeming_fee_total_accrued must have increased by the fees amount
    let mercurial_vault_depository_0_redeeming_fee_total_accrued_before =
        mercurial_vault_depository_0_before.redeeming_fee_total_accrued;
    let mercurial_vault_depository_0_redeeming_fee_total_accrued_after =
        mercurial_vault_depository_0_after.redeeming_fee_total_accrued;
    assert_eq!(
        mercurial_vault_depository_0_redeeming_fee_total_accrued_before
            + u128::from(mercurial_vault_depository_0_fees_amount),
        mercurial_vault_depository_0_redeeming_fee_total_accrued_after,
    );

    // mercurial_vault_depository_0.collateral_amount_deposited must have decreased by the withdrawn amount (equivalent to collateral_amount)
    let mercurial_vault_depository_0_collateral_amount_deposited_before =
        u64::try_from(mercurial_vault_depository_0_before.collateral_amount_deposited).unwrap();
    let mercurial_vault_depository_0_collateral_amount_deposited_after =
        u64::try_from(mercurial_vault_depository_0_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        mercurial_vault_depository_0_collateral_amount_deposited_before
            - mercurial_vault_depository_0_collateral_amount,
        mercurial_vault_depository_0_collateral_amount_deposited_after,
    );

    // user_collateral.amount must have increased by the withdrawn amount (equivalent to collateral_amount)
    assert_eq!(
        user_collateral_amount_before + total_collateral_amount,
        user_collateral_amount_after,
    );
    // user_redeemable.amount must have decreased by the redeemed amount (equivalent to redeemable_amount)
    assert_eq!(
        user_redeemable_amount_before - total_redeemable_amount,
        user_redeemable_amount_after,
    );

    // Done
    Ok(())
}
