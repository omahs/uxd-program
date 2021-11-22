
import { Controller, Depository, BTC_DECIMALS, SOL_DECIMALS, UXD, UXD_DECIMALS, Mango, createAndInitializeMango } from "@uxdprotocol/uxd-client";
import { BTC, WSOL } from "./identities";
import { workspace } from "@project-serum/anchor";
import { TXN_OPTS, provider } from "./provider";
import { NodeWallet } from "@project-serum/anchor/dist/cjs/provider";
import { ControllerAccount } from "@uxdprotocol/uxd-client/dist/types/uxd-interfaces";

const uxdProgram = workspace.Uxd;

export const slippageBase = 1000;

export let mango: Mango;

afterEach("", () => {
    console.log("\n=====================================\n");
});

before(" ======= [Suite 0 : Initialize mango (1 op)] ======= ", async () => {
    mango = await createAndInitializeMango(provider, `devnet`);
});

console.log(`UXD PROGRAM ID == ${uxdProgram.programId}`);

// Controller - The UXD mint keeper
export const controllerUXD = new Controller("UXD", UXD_DECIMALS, uxdProgram.programId);

// Depositories - An account that manage a Collateral mint for the controller
export const depositoryBTC = new Depository(BTC, "BTC", BTC_DECIMALS, uxdProgram.programId);
export const depositoryWSOL = new Depository(WSOL, "SOL", SOL_DECIMALS, uxdProgram.programId);

// Client interface to the Web3 call to `UXD-Program`
export const uxdClient = new UXD(provider, uxdProgram);

// Utils Calls ----------------------------------------------------------------

export async function collateralUIPriceInMangoQuote(user: NodeWallet, depository: Depository, mango: Mango): Promise<number> {
    return uxdClient.perpUIPriceInQuote(mango, depository)
}

export async function redeemableCirculatinSupply(controller: Controller): Promise<number> {
    return uxdClient.redeemableCirculatinSupply(controller, TXN_OPTS)
}

export async function getControllerAccount(controller: Controller): Promise<ControllerAccount> {
    return uxdClient.getControllerAccount(controller, TXN_OPTS)
}

// Permissionned Calls --------------------------------------------------------

export async function initializeController(authority: NodeWallet, controller: Controller): Promise<string> {
    return uxdClient.initializeController(controller, authority, TXN_OPTS);
}

export async function registerMangoDepository(authority: NodeWallet, controller: Controller, depository: Depository, mango: Mango): Promise<string> {
    return uxdClient.registerMangoDepository(controller, depository, mango, authority, TXN_OPTS);
}

export async function setRedeemableGlobalSupplyCap(authority: NodeWallet, controller: Controller, supplyCapUiAmount: number): Promise<string> {
    return uxdClient.setRedeemableGlobalSupplyCap(controller, authority, supplyCapUiAmount, TXN_OPTS);
}

// User Facing Permissionless Calls -------------------------------------------

export function mintWithMangoDepository(user: NodeWallet, slippage: number, collateralAmount: number, controller: Controller, depository: Depository, mango: Mango): Promise<string> {
    return uxdClient.mintWithMangoDepository(collateralAmount, slippage, controller, depository, mango, user, TXN_OPTS);
}

export function redeemFromMangoDepository(user: NodeWallet, slippage: number, amountRedeemable: number, controller: Controller, depository: Depository, mango: Mango): Promise<string> {
    return uxdClient.redeemFromMangoDepository(amountRedeemable, slippage, controller, depository, mango, user, TXN_OPTS);
}

// ----------------------------------------------------------------------------

before("PerpMarketConfig for BTC", async () => {
    const perpMarketConfigBTC = mango.getPerpMarketConfigFor(depositoryBTC.collateralMintSymbol);
    const perpMarketIndexBTC = perpMarketConfigBTC.marketIndex;
    const perpMarketBTC = await mango.group.loadPerpMarket(provider.connection, perpMarketIndexBTC, perpMarketConfigBTC.baseDecimals, perpMarketConfigBTC.quoteDecimals);
    console.log("--- Printing the Mango BTC perp market informations ---------------- ");
    console.log(perpMarketBTC.toPrettyString(mango.group, perpMarketConfigBTC));
});

before("PerpMarketConfig for WSOL", async () => {
    const perpMarketConfigWSOL = mango.getPerpMarketConfigFor(depositoryWSOL.collateralMintSymbol);
    const perpMarketIndexWSOL = perpMarketConfigWSOL.marketIndex;
    const perpMarketWSOL = await mango.group.loadPerpMarket(provider.connection, perpMarketIndexWSOL, perpMarketConfigWSOL.baseDecimals, perpMarketConfigWSOL.quoteDecimals);
    console.log("--- Printing the Mango BTC perp market informations ---------------- ");
    console.log(perpMarketWSOL.toPrettyString(mango.group, perpMarketConfigWSOL));
});