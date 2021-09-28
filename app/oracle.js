"use strict";

// DzmGB2YeeFbSL72cAxYtfQCQXzyyWW2xYPCJ1uSPtNiP

const anchor = require("@project-serum/anchor");
const idl = require("../target/idl/oracle.json");

const DEVNET = "http://128.0.113.156";
const TXN_COMMIT = "processed";
const TXN_OPTS = {commitment: TXN_COMMIT, preflightCommitment: TXN_COMMIT, skipPreflight: true};

const programKey = new anchor.web3.PublicKey(idl.metadata.address);
const devnetOracle = new anchor.web3.PublicKey("HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J");
const localOracle = anchor.utils.publicKey.findProgramAddressSync(["BTCUSD"], programKey)[0];

const provider = anchor.Provider.local();
anchor.setProvider(provider);

const program = new anchor.Program(idl, programKey);

async function main() {
    console.log("devnet btc price key:", devnetOracle.toString());
    console.log("local btc price key:", localOracle.toString());

    let devnetConn = new anchor.web3.Connection(DEVNET);
    let d = await devnetConn.getAccountInfo(devnetOracle);
    console.log(d);

    console.log("init");
    await program.rpc.init({
        accounts: {
            wallet: provider.wallet.publicKey,
            buffer: localOracle,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [provider.wallet.payer],
        options: TXN_OPTS,
    }).catch(() => null);

    var i = 0;
    while(true) {
        let j = i + 512 > d.data.length ? d.data.length : i + 512;

        console.log(`put [${i}..${j}]`);
        await program.rpc.put(new anchor.BN(i), d.data.slice(i, j), {
            accounts: {
                buffer: localOracle,
            },
            options: TXN_OPTS,
        });

        i += 512;
        if(i > d.data.length) break;
    }

    console.log("get");
    await program.rpc.get({
        accounts: {
            oracle: localOracle,
        },
        options: TXN_OPTS,
    });

}

main();
