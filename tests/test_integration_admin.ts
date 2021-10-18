import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { ControllerUXD } from "./utils/controller";
import { Depository } from "./utils/depository";
import { TXN_OPTS, utils, getRentExemption, connection, BTC, WSOL, admin, BTC_ORACLE, SOL_ORACLE } from "./utils/utils";
import { expect } from "chai";
import { MANGO_PROGRAM_ID } from "./utils/mango";
import { MangoAccountLayout } from "@blockworks-foundation/mango-client";

// Depositories - They represent the business object that tie a mint to a depository
export let depositoryBTC: Depository;
export let depositoryWSOL: Depository;

before("Setup mints and depositories", async () => {
  // GIVEN
  await utils.setupMango(); // Async fetch of mango group

  depositoryBTC = new Depository(BTC, "BTC", BTC_ORACLE);
  depositoryWSOL = new Depository(WSOL, "WSOL", SOL_ORACLE);
});

before("Standard Administrative flow for UXD Controller and depositories", () => {
  it("Create UXD Controller", async () => {
    // WHEN

    await ControllerUXD.rpc.new({
      accounts: {
        authority: admin.publicKey,
        state: ControllerUXD.statePda,
        uxdMint: ControllerUXD.mintPda,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [admin.payer],
      options: TXN_OPTS,
    });

    // THEN
    // XXX add asserts
  });

  it("Create BTC depository", async () => {
    await Depository.rpc.new(ControllerUXD.ProgramId, {
      accounts: {
        payer: admin.publicKey,
        state: depositoryBTC.statePda,
        redeemableMint: depositoryBTC.redeemableMintPda,
        programCoin: depositoryBTC.depositPda,
        coinMint: depositoryBTC.collateralMint,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [admin.payer],
      options: TXN_OPTS,
    });
    // Add some asserts ...
    depositoryBTC.info();
  });

  // it("Create SOL depository", async () => {
  //   if (await connection.getAccountInfo(depositoryWSOL.statePda)) {
  //     // We don't really want to have non exactly reproducible, but with the localnet pinned adresses
  //     // Temporary
  //     console.log("DepositoryWSOL already initialized...");
  //   } else {
  //     await Depository.rpc.new(ControllerUXD.ProgramId, {
  //       accounts: {
  //         payer: admin.publicKey,
  //         state: depositoryWSOL.statePda,
  //         redeemableMint: depositoryWSOL.redeemableMintPda,
  //         programCoin: depositoryWSOL.depositPda,
  //         coinMint: depositoryWSOL.collateralMint,
  //         rent: SYSVAR_RENT_PUBKEY,
  //         systemProgram: SystemProgram.programId,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [admin.payer],
  //       options: TXN_OPTS,
  //     });
  //   }
  //   // Add some asserts ...
  //   depositoryWSOL.info();
  // });

  it("Register BTC Depository with Controller", async () => {
    // Given
    const depositoryRecordPda = ControllerUXD.depositoryRecordPda(depositoryBTC.collateralMint);

    const createMangoAccountIx = SystemProgram.createAccount({
      programId: MANGO_PROGRAM_ID,
      space: MangoAccountLayout.span,
      lamports: await getRentExemption(MangoAccountLayout.span),
      fromPubkey: admin.publicKey,
      newAccountPubkey: depositoryBTC.mangoAccount.publicKey,
    });

    // WHEN
    await ControllerUXD.rpc.registerDepository(depositoryBTC.oraclePriceAccount, {
      accounts: {
        authority: admin.publicKey,
        state: ControllerUXD.statePda,
        depositoryRecord: depositoryRecordPda,
        depositoryState: depositoryBTC.statePda,
        coinMint: depositoryBTC.collateralMint,
        coinPassthrough: ControllerUXD.coinPassthroughPda(depositoryBTC.collateralMint),
        mangoGroup: utils.mango.group.publicKey,
        mangoAccount: depositoryBTC.mangoAccount.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        mangoProgram: MANGO_PROGRAM_ID,
      },
      signers: [admin.payer, depositoryBTC.mangoAccount],
      options: TXN_OPTS,
      instructions: [createMangoAccountIx],
    });
  });

  // it("Register SOL Depository with Controller", async () => {
  //   await ControllerUXD.rpc.registerDepository(depositorySOL.oraclePriceAccount, {
  //     accounts: {
  //       authority: admin.wallet.publicKey,
  //       state: ControllerUXD.statePda,
  //       depositoryRecord: ControllerUXD.depositoryRecordPda(depositorySOL.collateralMint),
  //       depositoryState: depositorySOL.statePda,
  //       coinMint: depositorySOL.collateralMint.publicKey,
  //       coinPassthrough: ControllerUXD.coinPassthroughPda(depositorySOL.collateralMint),
  //       rent: SYSVAR_RENT_PUBKEY,
  //       systemProgram: SystemProgram.programId,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       mangoProgram: MANGO_PROGRAM_ID,
  //     },
  //     signers: [admin.wallet],
  //     options: TXN_OPTS,
  //   });
  //   // Add some asserts ...
  // });
});

// It does fail, but how to play nice with Moche/Chai...
// it("Create BTC depository when already there should fail", async () => {
//   await Depository.rpc.new(ControllerUXD.ProgramId, {
//     accounts: {
//       payer: admin.wallet.publicKey,
//       state: depositoryBTC.statePda,
//       redeemableMint: depositoryBTC.redeemableMintPda,
//       programCoin: depositoryBTC.depositPda,
//       coinMint: depositoryBTC.collateralMint.publicKey,
//       rent: SYSVAR_RENT_PUBKEY,
//       systemProgram: SystemProgram.programId,
//       tokenProgram: TOKEN_PROGRAM_ID,
//     },
//     signers: [admin.wallet],
//     options: TXN_OPTS,
//   });
//   // Add some asserts ...
//   expect.fail("Should fail because the BTC depository is already initialized.");
// });
