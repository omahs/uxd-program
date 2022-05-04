import { Keypair, Signer } from "@solana/web3.js";
import { Controller, MangoDepository, SOL_DECIMALS, USDC_DECIMALS, UXD_DECIMALS, WSOL, USDC_DEVNET, BTC_DECIMALS, BTC_DEVNET, ETH_DECIMALS, ETH_DEVNET } from "@uxdprotocol/uxd-client";
import { authority, bank, slippageBase, uxdProgramId } from "./constants";
import { printDepositoryInfo, printUserInfo, transferAllSol, transferAllTokens, transferSol, transferTokens } from "./utils";
import { depositInsuranceMangoDepositoryTest } from "./cases/depositInsuranceMangoDepositoryTest";
import { registerMangoDepositoryTest } from "./cases/registerMangoDepositoryTest";
import { mango } from "./fixtures";
import { withdrawInsuranceMangoDepositoryTest } from "./cases/withdrawInsuranceMangoDepositoryTest";
import { mintWithMangoDepositoryTest } from "./cases/mintWithMangoDepositoryTest";
import { redeemFromMangoDepositoryTest } from "./cases/redeemFromMangoDepositoryTest";
import { initializeControllerTest } from "./cases/initializeControllerTest";
import { MangoDepositoryRebalancingSuiteParameters, mangoDepositoryRebalancingSuite } from "./suite/mangoDepositoryRebalancingSuite";

console.log(uxdProgramId.toString());
const mangoDepositorySOL = new MangoDepository(WSOL, "SOL", SOL_DECIMALS, USDC_DEVNET, "USDC", USDC_DECIMALS, uxdProgramId);
const mangoDepositoryBTC = new MangoDepository(BTC_DEVNET, "BTC", BTC_DECIMALS, USDC_DEVNET, "USDC", USDC_DECIMALS, uxdProgramId);
const mangoDepositoryETH = new MangoDepository(ETH_DEVNET, "ETH", ETH_DECIMALS, USDC_DEVNET, "USDC", USDC_DECIMALS, uxdProgramId);
const controller = new Controller("UXD", UXD_DECIMALS, uxdProgramId);
const payer = bank;
const slippage = 50; // 5%

console.log(`SOL 🥭🔗 'https://devnet.mango.markets/account?pubkey=${mangoDepositorySOL.mangoAccountPda}'`);
console.log(`BTC 🥭🔗 'https://devnet.mango.markets/account?pubkey=${mangoDepositoryBTC.mangoAccountPda}'`);
console.log(`ETH 🥭🔗 'https://devnet.mango.markets/account?pubkey=${mangoDepositoryETH.mangoAccountPda}'`);

beforeEach("\n", function () { console.log("=============================================\n\n") });

// Use SOL as it's the special case using more computing
describe("Integration tests SOL", function () {
    const user: Signer = new Keypair();

    this.beforeAll("Init and fund user (10 SOL and 10k usdc)", async function () {
        console.log("USER =>", user.publicKey.toString());
        await transferSol(10, bank, user.publicKey);
        await transferTokens(10000, USDC_DEVNET, USDC_DECIMALS, bank, user.publicKey);
    });

    describe("Init", async function () {
        it("Initialize Controller", async function () {
            await initializeControllerTest(authority, controller, payer);
        });

        it.skip(`Initialize ${mangoDepositorySOL.collateralMintSymbol} Depository`, async function () {
            await registerMangoDepositoryTest(authority, controller, mangoDepositorySOL, mango, payer);
        });
        it.skip(`Initialize ${mangoDepositoryBTC.collateralMintSymbol} Depository`, async function () {
            await registerMangoDepositoryTest(authority, controller, mangoDepositoryBTC, mango, payer);
        });
        it.skip(`Initialize ${mangoDepositoryETH.collateralMintSymbol} Depository`, async function () {
            await registerMangoDepositoryTest(authority, controller, mangoDepositoryETH, mango, payer);
        });

        it.skip(`Deposit 100 USDC of insurance`, async function () {
            await depositInsuranceMangoDepositoryTest(100, authority, controller, mangoDepositorySOL, mango);
        });

        it.skip(`Withdraw 1 USDC of insurance`, async function () {
            await withdrawInsuranceMangoDepositoryTest(1, authority, controller, mangoDepositorySOL, mango);
        });

        // it(`Mint 80 ${controller.redeemableMintSymbol} then redeem the outcome (${slippage / slippageBase * 100} % slippage)`, async function () {
        //     const mintedAmount = await mintWithMangoDepositoryTest(80, slippage, user, controller, depository, mango, payer);
        // });

    });

    describe("Test minting/redeeming", async function () {

        it.skip(`Mint 10 ${controller.redeemableMintSymbol} then redeem the outcome (${slippage / slippageBase * 100} % slippage)`, async function () {
            const perpPrice = await mangoDepositorySOL.getCollateralPerpPriceUI(mango);
            const amount = 10 / perpPrice;
            console.log("[🧾 amount", amount, mangoDepositorySOL.collateralMintSymbol, "]");
            const mintedAmount = await mintWithMangoDepositoryTest(amount, slippage, user, controller, mangoDepositorySOL, mango, payer);
            await redeemFromMangoDepositoryTest(mintedAmount, slippage, user, controller, mangoDepositorySOL, mango, payer);
        });

        it.skip(`Mint twice min mint trading size, then redeem them (${slippage / slippageBase * 100}% slippage)`, async function () {
            const minRedeemAmount = await mangoDepositorySOL.getMinRedeemSizeQuoteUI(mango);
            const minTradingSize = await mangoDepositorySOL.getMinTradingSizeCollateralUI(mango);

            await mintWithMangoDepositoryTest(minTradingSize * 2, slippage, user, controller, mangoDepositorySOL, mango, payer);
            await redeemFromMangoDepositoryTest(minRedeemAmount, slippage, user, controller, mangoDepositorySOL, mango, payer);
        });
    });

    // Note - Keep a mint/redeem before rebalancing so that it creates the necessary accounts for computing
    describe.skip("mangoDepositoryRebalancingSuite SOL", function () {
        const paramsRebalancing = new MangoDepositoryRebalancingSuiteParameters(slippage)
        mangoDepositoryRebalancingSuite(user, bank, controller, mangoDepositorySOL, paramsRebalancing);
    });

    describe.skip("info", async function () {
        it("info", async function () {
            await printUserInfo(user.publicKey, controller, mangoDepositorySOL);
            await printDepositoryInfo(controller, mangoDepositorySOL, mango);
        });
    });

    this.afterAll("Transfer funds back to bank", async function () {
        await transferAllSol(user, bank.publicKey);
        await transferAllTokens(USDC_DEVNET, USDC_DECIMALS, user, bank.publicKey);
    });
});