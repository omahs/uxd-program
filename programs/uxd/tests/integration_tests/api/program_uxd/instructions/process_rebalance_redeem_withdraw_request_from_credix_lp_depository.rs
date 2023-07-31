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

use crate::integration_tests::api::program_credix;
use crate::integration_tests::api::program_mercurial;
use crate::integration_tests::api::program_test_context;
use crate::integration_tests::api::program_uxd;

pub async fn process_rebalance_redeem_withdraw_request_from_credix_lp_depository(
    program_test_context: &mut ProgramTestContext,
    payer: &Keypair,
    collateral_mint: &Pubkey,
    credix_multisig_key: &Pubkey,
    profits_beneficiary_collateral: &Pubkey,
    expected_withdrawal_overflow_value: u64,
    expected_withdrawal_profits_amount: u64,
) -> Result<(), program_test_context::ProgramTestError> {
    // Find needed accounts
    let controller = program_uxd::accounts::find_controller_pda().0;
    let redeemable_mint = program_uxd::accounts::find_redeemable_mint_pda().0;

    // Find identity_depository accounts
    let identity_depository = program_uxd::accounts::find_identity_depository_pda().0;
    let identity_depository_collateral =
        program_uxd::accounts::find_identity_depository_collateral_vault_pda().0;

    // Find mercurial_vault_depository accounts
    let mercurial_base = program_mercurial::accounts::find_base();
    let mercurial_vault_depository_vault =
        program_mercurial::accounts::find_vault_pda(collateral_mint, &mercurial_base.pubkey()).0;
    let mercurial_vault_depository = program_uxd::accounts::find_mercurial_vault_depository_pda(
        collateral_mint,
        &mercurial_vault_depository_vault,
    )
    .0;

    // Find credix_lp_depository_marketplace accounts
    let credix_program_state_marketplace = program_credix::accounts::find_program_state_pda().0;
    let credix_market_seeds_marketplace = program_credix::accounts::find_market_seeds_marketplace();
    let credix_global_market_state_marketplace =
        program_credix::accounts::find_global_market_state_pda(&credix_market_seeds_marketplace).0;
    let credix_lp_depository_marketplace = program_uxd::accounts::find_credix_lp_depository_pda(
        collateral_mint,
        &credix_global_market_state_marketplace,
    )
    .0;
    let credix_shares_mint_marketplace =
        program_credix::accounts::find_lp_token_mint_pda(&credix_market_seeds_marketplace).0;
    let credix_signing_authority_marketplace =
        program_credix::accounts::find_signing_authority_pda(&credix_market_seeds_marketplace).0;
    let credix_liquidity_collateral_marketplace =
        program_credix::accounts::find_liquidity_pool_token_account(
            &credix_signing_authority_marketplace,
            collateral_mint,
        );
    let credix_treasury_marketplace = program_credix::accounts::find_treasury(credix_multisig_key);
    let credix_treasury_collateral_marketplace =
        program_credix::accounts::find_treasury_pool_token_account(
            &credix_treasury_marketplace,
            collateral_mint,
        );
    let credix_pass_marketplace = program_credix::accounts::find_credix_pass_pda(
        &credix_global_market_state_marketplace,
        &credix_lp_depository_marketplace,
    )
    .0;
    let credix_multisig_collateral_marketplace =
        spl_associated_token_account::get_associated_token_address(
            credix_multisig_key,
            collateral_mint,
        );
    let credix_lp_depository_marketplace_collateral =
        program_uxd::accounts::find_credix_lp_depository_collateral(
            &credix_lp_depository_marketplace,
            collateral_mint,
        );
    let credix_lp_depository_marketplace_shares =
        program_uxd::accounts::find_credix_lp_depository_shares(
            &credix_lp_depository_marketplace,
            &credix_shares_mint_marketplace,
        );

    // Find the credix_lp_depository_marketplace withdraw accounts
    let credix_latest_withdraw_epoch_idx_marketplace =
        program_test_context::read_account_anchor::<credix_client::GlobalMarketState>(
            program_test_context,
            &credix_global_market_state_marketplace,
        )
        .await?
        .latest_withdraw_epoch_idx;
    let credix_withdraw_epoch_marketplace = program_credix::accounts::find_withdraw_epoch_pda(
        &credix_global_market_state_marketplace,
        credix_latest_withdraw_epoch_idx_marketplace,
    )
    .0;
    let credix_withdraw_request_marketplaces = program_credix::accounts::find_withdraw_request_pda(
        &credix_global_market_state_marketplace,
        &credix_lp_depository_marketplace,
        credix_latest_withdraw_epoch_idx_marketplace,
    )
    .0;

    // Read state before
    let redeemable_mint_before =
        program_test_context::read_account_packed::<Mint>(program_test_context, &redeemable_mint)
            .await?;
    let controller_before =
        program_test_context::read_account_anchor::<Controller>(program_test_context, &controller)
            .await?;
    let credix_lp_depository_marketplace_before =
        program_test_context::read_account_anchor::<CredixLpDepository>(
            program_test_context,
            &credix_lp_depository_marketplace,
        )
        .await?;
    let identity_depository_before =
        program_test_context::read_account_anchor::<IdentityDepository>(
            program_test_context,
            &identity_depository,
        )
        .await?;
    let identity_depository_collateral_amount_before =
        program_test_context::read_account_packed::<Account>(
            program_test_context,
            &identity_depository_collateral,
        )
        .await?
        .amount;
    let profits_beneficiary_collateral_amount_before =
        program_test_context::read_account_packed::<Account>(
            program_test_context,
            profits_beneficiary_collateral,
        )
        .await?
        .amount;

    // Execute IX
    let accounts = uxd::accounts::RebalanceRedeemWithdrawRequestFromCredixLpDepository {
        payer: payer.pubkey(),
        controller,
        collateral_mint: *collateral_mint,
        depository: credix_lp_depository,
        depository_collateral: credix_lp_depository_collateral,
        depository_shares: credix_lp_depository_shares,
        credix_program_state,
        credix_global_market_state,
        credix_signing_authority,
        credix_liquidity_collateral,
        credix_shares_mint,
        credix_pass,
        credix_withdraw_epoch,
        credix_withdraw_request,
        credix_treasury_collateral,
        credix_multisig_key: *credix_multisig_key,
        credix_multisig_collateral,
        profits_beneficiary_collateral: *profits_beneficiary_collateral,
        identity_depository,
        identity_depository_collateral,
        mercurial_vault_depository,
        system_program: anchor_lang::system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        credix_program: credix_client::ID,
        rent: anchor_lang::solana_program::sysvar::rent::ID,
    };
    let payload = uxd::instruction::RebalanceRedeemWithdrawRequestFromCredixLpDepository {};
    let instruction = Instruction {
        program_id: uxd::id(),
        accounts: accounts.to_account_metas(None),
        data: payload.data(),
    };
    program_test_context::process_instruction(program_test_context, instruction, payer).await?;

    // Read state after
    let redeemable_mint_after =
        program_test_context::read_account_packed::<Mint>(program_test_context, &redeemable_mint)
            .await?;
    let controller_after =
        program_test_context::read_account_anchor::<Controller>(program_test_context, &controller)
            .await?;
    let credix_lp_depository_after =
        program_test_context::read_account_anchor::<CredixLpDepository>(
            program_test_context,
            &credix_lp_depository,
        )
        .await?;
    let identity_depository_after =
        program_test_context::read_account_anchor::<IdentityDepository>(
            program_test_context,
            &identity_depository,
        )
        .await?;
    let identity_depository_collateral_amount_after =
        program_test_context::read_account_packed::<Account>(
            program_test_context,
            &identity_depository_collateral,
        )
        .await?
        .amount;
    let profits_beneficiary_collateral_amount_after =
        program_test_context::read_account_packed::<Account>(
            program_test_context,
            profits_beneficiary_collateral,
        )
        .await?
        .amount;

    // redeemable_mint.supply must stay unchanged
    let redeemable_mint_supply_before = redeemable_mint_before.supply;
    let redeemable_mint_supply_after = redeemable_mint_after.supply;
    assert_eq!(redeemable_mint_supply_before, redeemable_mint_supply_after,);

    // controller.redeemable_circulating_supply must stay unchanged
    let redeemable_circulating_supply_before =
        u64::try_from(controller_before.redeemable_circulating_supply).unwrap();
    let redeemable_circulating_supply_after =
        u64::try_from(controller_after.redeemable_circulating_supply).unwrap();
    assert_eq!(
        redeemable_circulating_supply_before,
        redeemable_circulating_supply_after,
    );

    // credix_lp_depository.redeemable_amount_under_management must have decreased by the withdraw overflow
    let credix_lp_depository_redeemable_amount_under_management_before =
        u64::try_from(credix_lp_depository_before.redeemable_amount_under_management).unwrap();
    let credix_lp_depository_redeemable_amount_under_management_after =
        u64::try_from(credix_lp_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        credix_lp_depository_redeemable_amount_under_management_before
            - expected_withdrawal_overflow_value,
        credix_lp_depository_redeemable_amount_under_management_after,
    );

    // identity_depository.redeemable_amount_under_management must have increased by the withdraw overflow
    let identity_depository_redeemable_amount_under_management_before =
        u64::try_from(identity_depository_before.redeemable_amount_under_management).unwrap();
    let identity_depository_redeemable_amount_under_management_after =
        u64::try_from(identity_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        identity_depository_redeemable_amount_under_management_before
            + expected_withdrawal_overflow_value,
        identity_depository_redeemable_amount_under_management_after,
    );

    // credix_lp_depository.collateral_amount_deposited must have decreased by the withdraw overflow
    let credix_lp_depository_collateral_amount_deposited_before =
        u64::try_from(credix_lp_depository_before.collateral_amount_deposited).unwrap();
    let credix_lp_depository_collateral_amount_deposited_after =
        u64::try_from(credix_lp_depository_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        credix_lp_depository_collateral_amount_deposited_before
            - expected_withdrawal_overflow_value,
        credix_lp_depository_collateral_amount_deposited_after,
    );

    // identity_depository.collateral_amount_deposited must have increased by the withdraw overflow
    let identity_depository_collateral_amount_deposited_before =
        u64::try_from(identity_depository_before.collateral_amount_deposited).unwrap();
    let identity_depository_collateral_amount_deposited_after =
        u64::try_from(identity_depository_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        identity_depository_collateral_amount_deposited_before + expected_withdrawal_overflow_value,
        identity_depository_collateral_amount_deposited_after,
    );

    // credix_lp_depository.profits_amount_collected must have increased by the profits amount
    let profits_total_collected_before: u64 =
        u64::try_from(credix_lp_depository_before.profits_total_collected).unwrap();
    let profits_total_collected_after =
        u64::try_from(credix_lp_depository_after.profits_total_collected).unwrap();
    assert_eq!(
        profits_total_collected_before + expected_withdrawal_profits_amount,
        profits_total_collected_after,
    );

    // identity_depository_collateral.amount must have increased by the overflow amount
    assert_eq!(
        identity_depository_collateral_amount_before + expected_withdrawal_overflow_value,
        identity_depository_collateral_amount_after,
    );

    // profits_beneficiary_collateral.amount must have increased by the profits amount
    assert_eq!(
        profits_beneficiary_collateral_amount_before + expected_withdrawal_profits_amount,
        profits_beneficiary_collateral_amount_after,
    );

    // Done
    Ok(())
}
