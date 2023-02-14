use solana_program_test::ProgramTestContext;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

pub async fn process_instruction(
    program_test_context: &mut ProgramTestContext,
    instruction: Instruction,
    payer: &Keypair,
) -> Result<(), String> {
    println!(" -------- PROCESSING INSTRUCTION (no signers) --------");
    println!(
        " - instruction.program_id: {:?}",
        instruction.program_id.to_string()
    );
    println!(" - instruction.accounts: {:?}", instruction.accounts);
    println!(" - instruction.data: {:?}", instruction.data);
    let mut transaction: Transaction =
        Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.partial_sign(&[payer], program_test_context.last_blockhash);
    let result = program_test_context
        .banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.to_string());
    println!(" >>>> RESULT: {:?}", result);
    result
}