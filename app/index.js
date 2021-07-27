"use strict";

const fs = require("fs");
const anchor = require("@project-serum/anchor");
const spl = require("@solana/spl-token");

// these i have to change based on the whims of solana
const PROGRAM_ID = process.argv[2];
if(!PROGRAM_ID) throw "specify program id";
const TEST_MINT = "AG76P5h5aqw3QWFBhwKz8cFfmFMsHoDeVyhm8exDKh4R";
const MINT_DECIMAL = 9;

// this is theoretically constant everywhere
const TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ASSOC_TOKEN_PROGRAM_ID = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

const TXN_COMMIT = "processed";
const TXN_OPTS = {commitment: TXN_COMMIT, preflightCommitment: TXN_COMMIT, skipPreflight: false};

const programKey = new anchor.web3.PublicKey(PROGRAM_ID);
const depositMintKey = new anchor.web3.PublicKey(TEST_MINT);
const tokenProgramKey = new anchor.web3.PublicKey(TOKEN_PROGRAM_ID);
const assocTokenProgramKey = new anchor.web3.PublicKey(ASSOC_TOKEN_PROGRAM_ID);

const provider = anchor.Provider.local();
anchor.setProvider(provider);

// derives the canonical token account address for a given wallet and mint
async function findAssocTokenAddr(wallet, mint) {
    return (await anchor.web3.PublicKey.findProgramAddress([
        wallet.toBuffer(),
        tokenProgramKey.toBuffer(),
        mint.toBuffer(),
    ], assocTokenProgramKey))[0];
}

async function main() {
    let idl = JSON.parse(fs.readFileSync("/home/hana/work/soteria/solana-usds/target/idl/depository.json"));
    let program = new anchor.Program(idl, programKey);

    // anchor insists on including wallet addresses in derived accounts
    // so i cant use their fucntions intended for this
    let stateKey = (await anchor.web3.PublicKey.findProgramAddress([Buffer.from("STATE")], programKey))[0];
    let redeemableMintKey = (await anchor.web3.PublicKey.findProgramAddress([Buffer.from("REDEEMABLE")], programKey))[0];
    let depositAccountKey = (await anchor.web3.PublicKey.findProgramAddress([Buffer.from("DEPOSIT")], programKey))[0];

    console.log("program id:", PROGRAM_ID);
    console.log("program authority:", stateKey.toString());
    console.log("redeemable mint:", redeemableMintKey.toString());

    // standard spl associated accounts
    let walletCoinKey = await findAssocTokenAddr(provider.wallet.publicKey, depositMintKey);
    let walletRedeemableKey = await findAssocTokenAddr(provider.wallet.publicKey, redeemableMintKey);

    // set up the program
    // im deploying to a new address each time soo this is fine
    await program.rpc.new({
        accounts: {
            payer: provider.wallet.publicKey,
            state: stateKey,
            redeemableMint: redeemableMintKey,
            programCoin: depositAccountKey,
            coinMint: depositMintKey,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: tokenProgramKey,
            program: programKey,
        },
        signers: [provider.wallet.payer],
        options: TXN_OPTS,
    });

    console.log("initialized!");

    // ok now uhh i just wanna deposit and withdraw. may or may not have to set up the redeemable acct
    let redeemState = await provider.connection.getAccountInfo(walletRedeemableKey);

    // anchor will error if you pass [] or null lol
    var depositIxns = undefined;
    if(!redeemState) {
        depositIxns = [
            new anchor.web3.TransactionInstruction({
                keys: [
                    {pubkey: provider.wallet.publicKey, isSigner: true, isWritable: true},
                    {pubkey: walletRedeemableKey, isSigner: false, isWritable: true},
                    {pubkey: provider.wallet.publicKey, isSigner: false, isWritable: false},
                    {pubkey: redeemableMintKey, isSigner: false, isWritable: false},
                    {pubkey: anchor.web3.SystemProgram.programId, isSigner: false, isWritable: false},
                    {pubkey: tokenProgramKey, isSigner: false, isWritable: false},
                    {pubkey: anchor.web3.SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
                ],
                programId: assocTokenProgramKey,
                data: Buffer.alloc(0),
            }),
        ]
    }

    console.log("user token balance before deposit:", (await provider.connection.getTokenAccountBalance(walletCoinKey, TXN_COMMIT))["value"]["uiAmount"]);

    await program.rpc.deposit(new anchor.BN(1 * 10**MINT_DECIMAL), {
        accounts: {
            user: provider.wallet.publicKey,
            state: stateKey,
            programCoin: depositAccountKey,
            redeemableMint: redeemableMintKey,
            userCoin: walletCoinKey,
            userRedeemable: walletRedeemableKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: tokenProgramKey,
            program: programKey,
        },
        signers: [provider.wallet.payer],
        options: TXN_OPTS,
        instructions: depositIxns,
    });

    console.log("user token balance after deposit:", (await provider.connection.getTokenAccountBalance(walletCoinKey, TXN_COMMIT))["value"]["uiAmount"]);

}

main();
