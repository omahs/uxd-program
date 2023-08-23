use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use crate::integration_tests::api::program_credix;
use crate::integration_tests::api::program_test_context;

pub async fn process_initialize_program_state(
    program_runner: &mut dyn program_test_context::ProgramRunner,
    multisig: &Keypair,
) -> Result<(), program_test_context::ProgramTestError> {
    // Find needed accounts
    let program_state = program_credix::accounts::find_program_state_pda().0;
    let treasury = program_credix::accounts::find_treasury(&multisig.pubkey());

    // Execute IX
    let accounts = credix_client::accounts::InitializeProgramState {
        owner: multisig.pubkey(),
        program_state,
        system_program: anchor_lang::system_program::ID,
        rent: anchor_lang::solana_program::sysvar::rent::ID,
    };
    let payload = credix_client::instruction::InitializeProgramState {
        _credix_managers: [
            treasury, treasury, treasury, treasury, treasury, treasury, treasury, treasury,
            treasury, treasury,
        ],
        _credix_multisig_key: multisig.pubkey(),
    };
    let instruction = Instruction {
        program_id: credix_client::id(),
        accounts: accounts.to_account_metas(None),
        data: payload.data(),
    };
    program_test_context::process_instruction(program_runner, instruction, multisig).await
}
