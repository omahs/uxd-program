use solana_program_test::tokio;
use solana_sdk::clock::SECONDS_PER_DAY;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;

use uxd::instructions::EditControllerFields;
use uxd::instructions::EditCredixLpDepositoryFields;
use uxd::instructions::EditDepositoriesRoutingWeightBps;
use uxd::instructions::EditIdentityDepositoryFields;
use uxd::instructions::EditMercurialVaultDepositoryFields;

use crate::integration_tests::api::program_credix;
use crate::integration_tests::api::program_spl;
use crate::integration_tests::api::program_test_context;
use crate::integration_tests::api::program_uxd;
use crate::integration_tests::utils::ui_amount_to_native_amount;

#[tokio::test]
async fn test_credix_lp_depository_rebalance_no_overflow(
) -> Result<(), program_test_context::ProgramTestError> {
    // ---------------------------------------------------------------------
    // -- Phase 1
    // -- Setup basic context and accounts needed for this test suite
    // ---------------------------------------------------------------------

    let mut program_runner = program_test_context::create_program_test_context().await;

    // Fund payer
    let payer = Keypair::new();
    program_spl::instructions::process_lamports_airdrop(
        &mut program_runner,
        &payer.pubkey(),
        1_000_000_000_000,
    )
    .await?;

    // Hardcode mints decimals
    let collateral_mint_decimals = 6;
    let redeemable_mint_decimals = 6;

    // Important account keys
    let authority = Keypair::new();
    let collateral_mint = Keypair::new();
    let mercurial_vault_lp_mint = Keypair::new();
    let credix_multisig = Keypair::new();

    // Initialize basic UXD program state
    program_uxd::procedures::process_deploy_program(
        &mut program_runner,
        &payer,
        &authority,
        &collateral_mint,
        &mercurial_vault_lp_mint,
        &credix_multisig,
        collateral_mint_decimals,
        redeemable_mint_decimals,
    )
    .await?;

    // Main actor
    let user = Keypair::new();
    let profits_beneficiary = Keypair::new();

    // Create a collateral account for our user
    let user_collateral = program_spl::instructions::process_associated_token_account_get_or_init(
        &mut program_runner,
        &payer,
        &collateral_mint.pubkey(),
        &user.pubkey(),
    )
    .await?;
    // Create a redeemable account for our user
    let user_redeemable = program_spl::instructions::process_associated_token_account_get_or_init(
        &mut program_runner,
        &payer,
        &program_uxd::accounts::find_redeemable_mint_pda().0,
        &user.pubkey(),
    )
    .await?;

    // Create a collateral account for our profits_beneficiary
    let profits_beneficiary_collateral =
        program_spl::instructions::process_associated_token_account_get_or_init(
            &mut program_runner,
            &payer,
            &collateral_mint.pubkey(),
            &profits_beneficiary.pubkey(),
        )
        .await?;

    // Useful amounts used during testing scenario
    let amount_we_use_as_supply_cap =
        ui_amount_to_native_amount(50_000_000, redeemable_mint_decimals);

    let amount_of_collateral_airdropped_to_user =
        ui_amount_to_native_amount(1_000_000_000, collateral_mint_decimals);
    let amount_the_user_should_be_able_to_mint =
        ui_amount_to_native_amount(50_000_000, collateral_mint_decimals);

    // ---------------------------------------------------------------------
    // -- Phase 2
    // -- Prepare the program state to be ready,
    // -- Mint a bunch using credix to fill it up above its target
    // ---------------------------------------------------------------------

    // Airdrop collateral to our user
    program_spl::instructions::process_token_mint_to(
        &mut program_runner,
        &payer,
        &collateral_mint.pubkey(),
        &collateral_mint,
        &user_collateral,
        amount_of_collateral_airdropped_to_user,
    )
    .await?;

    // Set the controller cap and the weights
    program_uxd::instructions::process_edit_controller(
        &mut program_runner,
        &payer,
        &authority,
        &EditControllerFields {
            redeemable_global_supply_cap: Some(amount_we_use_as_supply_cap.into()),
            depositories_routing_weight_bps: Some(EditDepositoriesRoutingWeightBps {
                identity_depository_weight_bps: 0,
                mercurial_vault_depository_weight_bps: 0,
                credix_lp_depository_weight_bps: 100 * 100,
            }),
            router_depositories: None,
            outflow_limit_per_epoch_amount: None,
            outflow_limit_per_epoch_bps: None,
            slots_per_epoch: None,
        },
    )
    .await?;

    // Now we set the router depositories to the correct PDAs
    program_uxd::procedures::process_set_router_depositories(
        &mut program_runner,
        &payer,
        &authority,
        &collateral_mint.pubkey(),
    )
    .await?;

    // Set the identity_depository cap and make sure minting is not disabled
    program_uxd::instructions::process_edit_identity_depository(
        &mut program_runner,
        &payer,
        &authority,
        &EditIdentityDepositoryFields {
            redeemable_amount_under_management_cap: Some(amount_we_use_as_supply_cap.into()),
            minting_disabled: Some(false),
        },
    )
    .await?;

    // Set the mercurial_vault_depository cap and make sure minting is not disabled
    program_uxd::instructions::process_edit_mercurial_vault_depository(
        &mut program_runner,
        &payer,
        &authority,
        &collateral_mint.pubkey(),
        &EditMercurialVaultDepositoryFields {
            redeemable_amount_under_management_cap: Some(amount_we_use_as_supply_cap.into()),
            minting_fee_in_bps: Some(100),
            redeeming_fee_in_bps: Some(100),
            minting_disabled: Some(false),
            profits_beneficiary_collateral: Some(profits_beneficiary_collateral),
        },
    )
    .await?;

    // Set the credix_lp_depository cap and make sure minting is not disabled
    program_uxd::instructions::process_edit_credix_lp_depository(
        &mut program_runner,
        &payer,
        &authority,
        &collateral_mint.pubkey(),
        &EditCredixLpDepositoryFields {
            redeemable_amount_under_management_cap: Some(amount_we_use_as_supply_cap.into()),
            minting_fee_in_bps: Some(100),
            redeeming_fee_in_bps: Some(100),
            minting_disabled: Some(false),
            profits_beneficiary_collateral: Some(profits_beneficiary_collateral),
        },
    )
    .await?;

    // Minting on credix should work now that everything is set
    program_uxd::instructions::process_mint_with_credix_lp_depository(
        &mut program_runner,
        &payer,
        &authority,
        &collateral_mint.pubkey(),
        &user,
        &user_collateral,
        &user_redeemable,
        amount_the_user_should_be_able_to_mint,
    )
    .await?;

    // ---------------------------------------------------------------------
    // -- Phase 2
    // -- Do a complete rebalance, but no overflow should be needed, only profits
    // ---------------------------------------------------------------------

    // Create an epoch (done by credix team usually)
    program_credix::instructions::process_create_withdraw_epoch(
        &mut program_runner,
        &credix_multisig,
        1,
    )
    .await?;

    // Since the epoch was just created it should be available to create a WithdrawRequest
    program_uxd::instructions::process_rebalance_create_withdraw_request_from_credix_lp_depository(
        &mut program_runner,
        &payer,
        &collateral_mint.pubkey(),
    )
    .await?;

    // Pretend 3 days have passed (the time for the request period)
    program_test_context::move_clock_forward(&mut program_runner, 3 * SECONDS_PER_DAY, 1).await?;

    // Set the epoch's locked liquidity (done by credix team usually)
    program_credix::instructions::process_set_locked_liquidity(
        &mut program_runner,
        &credix_multisig,
        &collateral_mint.pubkey(),
    )
    .await?;

    // Executing the rebalance request should now work as intended because we are in the execute period
    let expected_credix_profits = amount_the_user_should_be_able_to_mint / 100; // minting fees 1%
    program_uxd::instructions::process_rebalance_redeem_withdraw_request_from_credix_lp_depository(
        &mut program_runner,
        &payer,
        &collateral_mint.pubkey(),
        &credix_multisig.pubkey(),
        &profits_beneficiary_collateral,
        0, // No overflow, no need to withdraw anything
        expected_credix_profits // Profits should be withdrawn
         - 1, // Precision loss expected to be taken out of the profits
    )
    .await?;

    // Done
    Ok(())
}
