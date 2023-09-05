use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use solana_address_lookup_table_program::state::AddressLookupTable;
use solana_program::message::v0;
use solana_program::message::VersionedMessage;
use solana_program_test::ProgramTestContext;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use spl_token::state::Account;
use spl_token::state::Mint;
use tarpc::context;

use uxd::state::Controller;
use uxd::state::CredixLpDepository;
use uxd::state::IdentityDepository;
use uxd::state::MercurialVaultDepository;
use uxd::utils::calculate_amount_less_fees;

use crate::integration_tests::api::program_credix;
use crate::integration_tests::api::program_mercurial;
use crate::integration_tests::api::program_test_context;
use crate::integration_tests::api::program_uxd;

#[allow(clippy::too_many_arguments)]
pub async fn process_mint(
    program_test_context: &mut ProgramTestContext,
    payer: &Keypair,
    collateral_mint: &Pubkey,
    mercurial_vault_depository_vault_lp_mint: &Pubkey,
    user: &Keypair,
    user_collateral: &Pubkey,
    user_redeemable: &Pubkey,
    collateral_amount: u64,
    expected_identity_depository_collateral_amount: u64,
    expected_mercurial_vault_depository_collateral_amount: u64,
    expected_credix_lp_depository_collateral_amount: u64,
) -> Result<(), program_test_context::ProgramTestError> {
    // Find needed accounts
    let controller = program_uxd::accounts::find_controller_pda().0;
    let redeemable_mint = program_uxd::accounts::find_redeemable_mint_pda().0;

    // Find identity depository related accounts
    let identity_depository = program_uxd::accounts::find_identity_depository_pda().0;
    let identity_depository_collateral_vault =
        program_uxd::accounts::find_identity_depository_collateral_vault_pda().0;

    // Find mercurial related accounts
    let mercurial_base = program_mercurial::accounts::find_base();
    let mercurial_vault_depository_vault =
        program_mercurial::accounts::find_vault_pda(collateral_mint, &mercurial_base.pubkey()).0;
    let mercurial_vault_depository = program_uxd::accounts::find_mercurial_vault_depository_pda(
        collateral_mint,
        &mercurial_vault_depository_vault,
    )
    .0;
    let mercurial_vault_depository_lp_token_vault =
        program_uxd::accounts::find_mercurial_vault_depository_lp_token_vault_pda(
            collateral_mint,
            &mercurial_vault_depository_vault,
        )
        .0;
    let mercurial_vault_depository_collateral_token_safe =
        program_mercurial::accounts::find_token_vault_pda(&mercurial_vault_depository_vault).0;

    // Find credix related accounts
    let credix_market_seeds = program_credix::accounts::find_market_seeds();
    let credix_lp_depository_global_market_state =
        program_credix::accounts::find_global_market_state_pda(&credix_market_seeds).0;
    let credix_lp_depository = program_uxd::accounts::find_credix_lp_depository_pda(
        collateral_mint,
        &credix_lp_depository_global_market_state,
    )
    .0;
    let credix_lp_depository_shares_mint =
        program_credix::accounts::find_lp_token_mint_pda(&credix_market_seeds).0;
    let credix_lp_depository_signing_authority =
        program_credix::accounts::find_signing_authority_pda(&credix_market_seeds).0;
    let credix_lp_depository_liquidity_collateral =
        program_credix::accounts::find_liquidity_pool_token_account(
            &credix_lp_depository_signing_authority,
            collateral_mint,
        );
    let credix_lp_depository_pass = program_credix::accounts::find_credix_pass_pda(
        &credix_lp_depository_global_market_state,
        &credix_lp_depository,
    )
    .0;
    let credix_lp_depository_collateral =
        program_uxd::accounts::find_credix_lp_depository_collateral(
            &credix_lp_depository,
            collateral_mint,
        );
    let credix_lp_depository_shares = program_uxd::accounts::find_credix_lp_depository_shares(
        &credix_lp_depository,
        &credix_lp_depository_shares_mint,
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
    let mercurial_vault_depository_before = program_test_context::read_account_anchor::<
        MercurialVaultDepository,
    >(program_test_context, &mercurial_vault_depository)
    .await?;
    let credix_lp_depository_before =
        program_test_context::read_account_anchor::<CredixLpDepository>(
            program_test_context,
            &credix_lp_depository,
        )
        .await?;

    let user_collateral_amount_before =
        program_test_context::read_account_packed::<Account>(program_test_context, user_collateral)
            .await?
            .amount;
    let user_redeemable_amount_before =
        program_test_context::read_account_packed::<Account>(program_test_context, user_redeemable)
            .await?
            .amount;

    let recent_slot = program_test_context
        .banks_client
        .get_slot_with_context(context::current(), CommitmentLevel::Finalized)
        .await
        .map_err(program_test_context::ProgramTestError::BanksClient)?;

    let (create_ix, table_pk) =
        solana_address_lookup_table_program::instruction::create_lookup_table(
            payer.pubkey(),
            payer.pubkey(),
            recent_slot,
        );

    program_test_context::process_instruction(program_test_context, create_ix, payer).await?;

    let dudu = program_test_context::read_account_data(program_test_context, &table_pk).await?;
    let lookup = AddressLookupTable::deserialize(&dudu);

    // Execute IX
    let accounts = uxd::accounts::Mint {
        user: user.pubkey(),
        payer: payer.pubkey(),
        controller,
        collateral_mint: *collateral_mint,
        redeemable_mint,
        user_collateral: *user_collateral,
        user_redeemable: *user_redeemable,
        identity_depository,
        identity_depository_collateral_vault,
        mercurial_vault_depository,
        mercurial_vault_depository_vault,
        mercurial_vault_depository_vault_lp_mint: *mercurial_vault_depository_vault_lp_mint,
        mercurial_vault_depository_lp_token_vault,
        mercurial_vault_depository_collateral_token_safe,
        credix_lp_depository,
        credix_lp_depository_collateral,
        credix_lp_depository_shares,
        credix_lp_depository_pass,
        credix_lp_depository_global_market_state,
        credix_lp_depository_signing_authority,
        credix_lp_depository_liquidity_collateral,
        credix_lp_depository_shares_mint,
        system_program: anchor_lang::system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        mercurial_vault_program: mercurial_vault::ID,
        credix_program: credix_client::ID,
        uxd_program: uxd::ID,
        rent: anchor_lang::solana_program::sysvar::rent::ID,
        unused0: uxd::ID,
        unused1: uxd::ID,
        unused2: uxd::ID,
        unused3: uxd::ID,
        unused4: uxd::ID,
        unused5: uxd::ID,
        unused6: uxd::ID,
        unused7: uxd::ID,
        unused8: uxd::ID,
    };

    let accounts_keys = accounts
        .to_account_metas(None)
        .iter()
        .map(|e| e.pubkey)
        .collect::<Vec<Pubkey>>();

    let extend_ix = solana_address_lookup_table_program::instruction::extend_lookup_table(
        table_pk,
        payer.pubkey(),
        Some(payer.pubkey()),
        accounts_keys.clone(),
    );

    program_test_context::process_instruction(program_test_context, extend_ix, payer).await?;

    let payload = uxd::instruction::Mint { collateral_amount };
    let instruction = Instruction {
        program_id: uxd::id(),
        accounts: accounts.to_account_metas(None),
        data: payload.data(),
    };

    let blockhash = program_test_context.last_blockhash;

    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            v0::Message::try_compile(
                &payer.pubkey(),
                &[instruction],
                &[
                    solana_program::address_lookup_table_account::AddressLookupTableAccount {
                        key: table_pk,
                        addresses: accounts_keys.clone(),
                    },
                ],
                blockhash,
            )
            .map_err(program_test_context::ProgramTestError::Compile)?,
        ),
        &[payer, user],
    )
    .map_err(program_test_context::ProgramTestError::Signer)?;

    program_test_context::process_transaction(program_test_context, tx).await?;

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
    let mercurial_vault_depository_after = program_test_context::read_account_anchor::<
        MercurialVaultDepository,
    >(program_test_context, &mercurial_vault_depository)
    .await?;
    let credix_lp_depository_after =
        program_test_context::read_account_anchor::<CredixLpDepository>(
            program_test_context,
            &credix_lp_depository,
        )
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
    let identity_depository_redeemable_amount = expected_identity_depository_collateral_amount;

    // Compute mercurial_vault_depository amounts
    let mercurial_vault_depository_redeemable_amount = calculate_amount_less_fees(
        expected_mercurial_vault_depository_collateral_amount,
        mercurial_vault_depository_before.minting_fee_in_bps,
    )
    .map_err(program_test_context::ProgramTestError::Anchor)?;
    let mercurial_vault_depository_fees_amount =
        expected_mercurial_vault_depository_collateral_amount
            - mercurial_vault_depository_redeemable_amount;

    // Compute credix_lp_depository amounts
    let credix_lp_depository_collateral_amount_after_precision_loss =
    program_uxd::instructions::process_mint_with_credix_lp_depository_collateral_amount_after_precision_loss(
            program_test_context,
            collateral_mint,
            expected_credix_lp_depository_collateral_amount,
        )
        .await?;
    let credix_lp_depository_redeemable_amount = calculate_amount_less_fees(
        credix_lp_depository_collateral_amount_after_precision_loss,
        credix_lp_depository_before.minting_fee_in_bps,
    )
    .map_err(program_test_context::ProgramTestError::Anchor)?;
    let credix_lp_depository_fees_amount =
        credix_lp_depository_collateral_amount_after_precision_loss
            - credix_lp_depository_redeemable_amount;

    // Compute total amounts
    let total_collateral_amount = expected_identity_depository_collateral_amount
        + expected_mercurial_vault_depository_collateral_amount
        + expected_credix_lp_depository_collateral_amount;
    let total_redeemable_amount = identity_depository_redeemable_amount
        + mercurial_vault_depository_redeemable_amount
        + credix_lp_depository_redeemable_amount;

    // redeemable_mint.supply must have increased by the minted amount (equivalent to redeemable_amount)
    let redeemable_mint_supply_before = redeemable_mint_before.supply;
    let redeemable_mint_supply_after = redeemable_mint_after.supply;
    assert_eq!(
        redeemable_mint_supply_before + total_redeemable_amount,
        redeemable_mint_supply_after,
    );

    // controller.redeemable_circulating_supply must have increased by the minted amount (equivalent to redeemable_amount)
    let redeemable_circulating_supply_before =
        u64::try_from(controller_before.redeemable_circulating_supply).unwrap();
    let redeemable_circulating_supply_after =
        u64::try_from(controller_after.redeemable_circulating_supply).unwrap();
    assert_eq!(
        redeemable_circulating_supply_before + total_redeemable_amount,
        redeemable_circulating_supply_after,
    );

    // identity_depository.redeemable_amount_under_management must have increased by the minted amount (equivalent to redeemable_amount)
    let identity_depository_redeemable_amount_under_management_before =
        u64::try_from(identity_depository_before.redeemable_amount_under_management).unwrap();
    let identity_depository_redeemable_amount_under_management_after =
        u64::try_from(identity_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        identity_depository_redeemable_amount_under_management_before
            + identity_depository_redeemable_amount,
        identity_depository_redeemable_amount_under_management_after,
    );

    // identity_depository.collateral_amount_deposited must have increased by the deposited amount (equivalent to collateral_amount)
    let identity_depository_collateral_amount_deposited_before =
        u64::try_from(identity_depository_before.collateral_amount_deposited).unwrap();
    let identity_depository_collateral_amount_deposited_after =
        u64::try_from(identity_depository_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        identity_depository_collateral_amount_deposited_before
            + expected_identity_depository_collateral_amount,
        identity_depository_collateral_amount_deposited_after,
    );

    // mercurial_vault_depository.redeemable_amount_under_management must have increased by the minted amount (equivalent to redeemable_amount)
    let mercurial_vault_depository_redeemable_amount_under_management_before =
        u64::try_from(mercurial_vault_depository_before.redeemable_amount_under_management)
            .unwrap();
    let mercurial_vault_depository_redeemable_amount_under_management_after =
        u64::try_from(mercurial_vault_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        mercurial_vault_depository_redeemable_amount_under_management_before
            + mercurial_vault_depository_redeemable_amount,
        mercurial_vault_depository_redeemable_amount_under_management_after,
    );

    // mercurial_vault_depository.minting_fee_total_accrued must have increased by the fees amount
    let mercurial_vault_depository_minting_fee_total_accrued_before =
        mercurial_vault_depository_before.minting_fee_total_accrued;
    let mercurial_vault_depository_minting_fee_total_accrued_after =
        mercurial_vault_depository_after.minting_fee_total_accrued;
    assert_eq!(
        mercurial_vault_depository_minting_fee_total_accrued_before
            + u128::from(mercurial_vault_depository_fees_amount),
        mercurial_vault_depository_minting_fee_total_accrued_after,
    );

    // mercurial_vault_depository.collateral_amount_deposited must have increased by the deposited amount (equivalent to collateral_amount)
    let mercurial_vault_depository_collateral_amount_deposited_before =
        u64::try_from(mercurial_vault_depository_before.collateral_amount_deposited).unwrap();
    let mercurial_vault_depository_collateral_amount_deposited_after =
        u64::try_from(mercurial_vault_depository_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        mercurial_vault_depository_collateral_amount_deposited_before
            + expected_mercurial_vault_depository_collateral_amount,
        mercurial_vault_depository_collateral_amount_deposited_after,
    );

    // credix_lp_depository.redeemable_amount_under_management must have increased by the minted amount (equivalent to redeemable_amount)
    let credix_lp_depository_redeemable_amount_under_management_before =
        u64::try_from(credix_lp_depository_before.redeemable_amount_under_management).unwrap();
    let credix_lp_depository_redeemable_amount_under_management_after =
        u64::try_from(credix_lp_depository_after.redeemable_amount_under_management).unwrap();
    assert_eq!(
        credix_lp_depository_redeemable_amount_under_management_before
            + credix_lp_depository_redeemable_amount,
        credix_lp_depository_redeemable_amount_under_management_after,
    );

    // credix_lp_depository.collateral_amount_deposited must have increased by the deposited amount (equivalent to collateral_amount)
    let credix_lp_depository_collateral_amount_deposited_before =
        u64::try_from(credix_lp_depository_before.collateral_amount_deposited).unwrap();
    let credix_lp_depository_collateral_amount_deposited_after =
        u64::try_from(credix_lp_depository_after.collateral_amount_deposited).unwrap();
    assert_eq!(
        credix_lp_depository_collateral_amount_deposited_before
            + expected_credix_lp_depository_collateral_amount,
        credix_lp_depository_collateral_amount_deposited_after,
    );

    // credix_lp_depository.minting_fee_total_accrued must have increased by the fees amount
    let credix_lp_depository_minting_fee_total_accrued_before =
        credix_lp_depository_before.minting_fee_total_accrued;
    let credix_lp_depository_minting_fee_total_accrued_after =
        credix_lp_depository_after.minting_fee_total_accrued;
    assert_eq!(
        credix_lp_depository_minting_fee_total_accrued_before
            + u128::from(credix_lp_depository_fees_amount),
        credix_lp_depository_minting_fee_total_accrued_after,
    );

    // user_collateral.amount must have decreased by the deposited amount (equivalent to collateral_amount)
    assert_eq!(
        user_collateral_amount_before - total_collateral_amount,
        user_collateral_amount_after,
    );
    // user_redeemable.amount must have increased by the minted amount (equivalent to redeemable_amount)
    assert_eq!(
        user_redeemable_amount_before + total_redeemable_amount,
        user_redeemable_amount_after,
    );

    // Done
    Ok(())
}
