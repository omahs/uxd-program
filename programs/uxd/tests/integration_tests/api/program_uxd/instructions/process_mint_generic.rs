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

use crate::integration_tests::api::program_credix;
use crate::integration_tests::api::program_mercurial;
use crate::integration_tests::api::program_spl;
use crate::integration_tests::api::program_test_context;
use crate::integration_tests::api::program_uxd;

pub async fn process_mint_generic(
    program_test_context: &mut ProgramTestContext,
    payer: &Keypair,
    collateral_mint: &Pubkey,
    mercurial_vault_depository_0_vault_lp_mint: &Pubkey,
    user: &Keypair,
    user_collateral: &Pubkey,
    user_redeemable: &Pubkey,
    collateral_amount: u64,
    expected_identity_depository_redeemable_amount: u64,
    expected_mercurial_vault_depository_0_redeemable_amount: u64,
    expected_credix_lp_depository_0_redeemable_amount: u64,
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
    let credix_lp_depository_0_shares_mint =
        program_credix::accounts::find_lp_token_mint_pda(&credix_market_seeds_0).0;
    let credix_lp_depository_0_signing_authority =
        program_credix::accounts::find_signing_authority_pda(&credix_market_seeds_0).0;
    let credix_lp_depository_0_liquidity_collateral =
        program_credix::accounts::find_liquidity_pool_token_account(
            &credix_lp_depository_0_signing_authority,
            collateral_mint,
        );
    let credix_lp_depository_0_pass = program_credix::accounts::find_credix_pass_pda(
        &credix_lp_depository_0_global_market_state,
        &credix_lp_depository_0,
    )
    .0;
    let credix_lp_depository_0_collateral =
        program_uxd::accounts::find_credix_lp_depository_collateral(
            &credix_lp_depository_0,
            collateral_mint,
        );
    let credix_lp_depository_0_shares = program_uxd::accounts::find_credix_lp_depository_shares(
        &credix_lp_depository_0,
        &credix_lp_depository_0_shares_mint,
    );

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
    let accounts = uxd::accounts::MintGeneric {
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
        credix_lp_depository_0_collateral,
        credix_lp_depository_0_shares,
        credix_lp_depository_0_pass,
        credix_lp_depository_0_global_market_state,
        credix_lp_depository_0_signing_authority,
        credix_lp_depository_0_liquidity_collateral,
        credix_lp_depository_0_shares_mint,
        system_program: anchor_lang::system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        mercurial_vault_program: mercurial_vault::ID,
        credix_program: credix_client::ID,
        rent: anchor_lang::solana_program::sysvar::rent::ID,
    };
    let payload = uxd::instruction::MintGeneric { collateral_amount };
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

    // Compute the expected redeemable amount
    let redeemable_amount = expected_identity_depository_redeemable_amount
        + expected_mercurial_vault_depository_0_redeemable_amount
        + expected_credix_lp_depository_0_redeemable_amount;

    // redeemable_mint.supply must have increased by the minted amount (equivalent to redeemable_amount)
    let redeemable_mint_supply_before = redeemable_mint_before.supply;
    let redeemable_mint_supply_after = redeemable_mint_after.supply;
    assert_eq!(
        redeemable_mint_supply_before + redeemable_amount,
        redeemable_mint_supply_after,
    );

    // controller.redeemable_circulating_supply must have increased by the minted amount (equivalent to redeemable_amount)
    let redeemable_circulating_supply_before =
        u64::try_from(controller_before.redeemable_circulating_supply).unwrap();
    let redeemable_circulating_supply_after =
        u64::try_from(controller_after.redeemable_circulating_supply).unwrap();
    assert_eq!(
        redeemable_circulating_supply_before + redeemable_amount,
        redeemable_circulating_supply_after,
    );

    // identity_depository.redeemable_amount_under_management must have increased by the minted amount (equivalent to redeemable_amount)
    let identity_depository_redeemable_amount_under_management_before =
        u64::try_from(identity_depository_before.redeemable_amount_under_management).unwrap();
    let identity_depository_redeemable_amount_under_management_after =
        u64::try_from(identity_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        identity_depository_redeemable_amount_under_management_before
            + expected_identity_depository_redeemable_amount,
        identity_depository_redeemable_amount_under_management_after,
    );

    // mercurial_vault_depository_0.redeemable_amount_under_management must have increased by the minted amount (equivalent to redeemable_amount)
    let mercurial_vault_depository_0_redeemable_amount_under_management_before =
        u64::try_from(mercurial_vault_depository_0_before.redeemable_amount_under_management)
            .unwrap();
    let mercurial_vault_depository_0_redeemable_amount_under_management_after =
        u64::try_from(mercurial_vault_depository_0_after.redeemable_amount_under_management)
            .unwrap();
    assert_eq!(
        mercurial_vault_depository_0_redeemable_amount_under_management_before
            + expected_mercurial_vault_depository_0_redeemable_amount,
        mercurial_vault_depository_0_redeemable_amount_under_management_after,
    );

    // credix_lp_depository_0.redeemable_amount_under_management must have increased by the minted amount (equivalent to redeemable_amount)
    let credix_lp_depository_0_redeemable_amount_under_management_before =
        u64::try_from(credix_lp_depository_0_before.redeemable_amount_under_management).unwrap();
    let credix_lp_depository_0_redeemable_amount_under_management_after =
        u64::try_from(credix_lp_depository_0_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        credix_lp_depository_0_redeemable_amount_under_management_before
            + expected_credix_lp_depository_0_redeemable_amount,
        credix_lp_depository_0_redeemable_amount_under_management_after,
    );

    // user_collateral.amount must have decreased by the deposited amount (equivalent to collateral_amount)
    assert_eq!(
        user_collateral_amount_before - collateral_amount,
        user_collateral_amount_after,
    );
    // user_redeemable.amount must have increased by the minted amount (equivalent to redeemable_amount)
    assert_eq!(
        user_redeemable_amount_before + redeemable_amount,
        user_redeemable_amount_after,
    );

    // Done
    Ok(())
}
