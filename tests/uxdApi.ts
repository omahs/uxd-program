
import { Controller, Depository, BTC_DECIMALS, SOL_DECIMALS, UXD, UXD_DECIMALS, Mango } from "@uxdprotocol/uxd-client";
import { BTC, WSOL } from "./identities";
import { workspace } from "@project-serum/anchor";
import { TXN_OPTS, provider } from "./provider";
import { NodeWallet } from "@project-serum/anchor/dist/cjs/provider";

const uxdProgram = workspace.Uxd;
const uxdProgramId = uxdProgram.programId;

console.log(`UXD PROGRAM ID == ${uxdProgramId}`);

// Controller - The UXD mint keeper, authority
export const controllerUXD = new Controller("UXD", UXD_DECIMALS, uxdProgramId);

// Depositories - An account that manage a Collateral mint for the controller
export const depositoryBTC = new Depository(BTC, "BTC", BTC_DECIMALS, uxdProgramId);
export const depositoryWSOL = new Depository(WSOL, "SOL", SOL_DECIMALS, uxdProgramId);

// Interface to the Web3 call to `UXD-Program`
export const uxd = new UXD(provider, uxdProgram);

// Permissionned Calls --------------------------------------------------------

export async function initializeControllerIfNeeded(authority: NodeWallet, controller: Controller, mango: Mango): Promise<string> {
    if (await provider.connection.getAccountInfo(controller.pda)) {
        console.log("Already initialized.");
    } else {
        return uxd.initializeController(controller, authority, TXN_OPTS);
    }
}

export async function registerMangoDepositoryIfNeeded(authority: NodeWallet, controller: Controller, depository: Depository, mango: Mango): Promise<string> {
    if (await provider.connection.getAccountInfo(depository.mangoAccountPda)) {
        console.log("Already registered.");
    } else {
        return uxd.registerMangoDepository(controller, depository, mango, authority, TXN_OPTS);
    }
}

// User Facing Permissionless Calls -------------------------------------------

export function mintWithMangoDepository(user: NodeWallet, slippage: number, collateralAmount: number, controller: Controller, depository: Depository, mango: Mango): Promise<string> {
    return uxd.mintWithMangoDepository(collateralAmount, slippage, controller, depository, mango, user, TXN_OPTS);
}

export function redeemFromMangoDepository(user: NodeWallet, slippage: number, amountRedeemable: number, controller: Controller, depository: Depository, mango: Mango): Promise<string> {
    return uxd.redeemFromMangoDepository(amountRedeemable, slippage, controller, depository, mango, user, TXN_OPTS);
}