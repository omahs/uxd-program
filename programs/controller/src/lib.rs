use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token::{self, Mint, TokenAccount, MintTo, Transfer};
use solana_program::{ system_program as system, program::invoke_signed };
use spl_token::instruction::{ initialize_account, initialize_mint };

// placeholder for figuring out best way
use mango_tester::{MangoTester, InitMangoAccount};

const MINT_SPAN: usize = 82;
const ACCOUNT_SPAN: usize = 165;
const MINT_DECIMAL: u8 = 9;

const STATE_SEED:       &[u8] = b"STATE";
const UXD_SEED:         &[u8] = b"STABLECOIN";
const RECORD_SEED:      &[u8] = b"RECORD";
const PASSTHROUGH_SEED: &[u8] = b"PASSTHROUGH";

#[program]
pub mod controller {
    use super::*;

    // MANGO API IN BRIEF
    // shit we care about:
    // * init account (good for a whole group aka a set of markets that can be xmargined)
    // * deposit (coin into account)
    // * withdraw (coin from account)
    // * place perp order (self explanatory. order type comes from serum i think)
    // * cancel perp order (their id and our id versions exist)
    // * settle pnl (takes two accounts and trues up)
    // settle is necessary but kinda weird in that like, you need to find a loser to match your winner
    //
    // shit we might:
    // * add to basket ("add a spot market to account basket" never made clear wtf this is)
    // * borrow (unclear if we need to borrow to short? prolly not...)
    // * place spot order (this is just a serum passthrough)
    // * cancel spot order (as above)
    // * settle funds ("settle funds from serum dex open orders" maybe just serum passthrough?)
    // * settle borrow (only if we use borrow
    // the point of serum calls is they can use the money in mango accounts
    // but... i dont think we need to mess w spot
    //
    // flow... user deposits btc, we send to mango
    // open a equiv sized short position sans whatever amount for liquidation protection
    // once the position is open it theoretically has a fix dollar value
    // (sans execution risk, sans funding, sans liquidation buffer)
    // this is the amount of uxd we mint and return to the user
    // then redemption of uxd for the underlying means we... burn uxd
    // close out an equivalent amount of position in the coin they want
    // settle pnl, withdraw coin, deliver to depository, give user redeemables
    // important that all trasaction costs and price differences *must* be passed onto the user
    // otherwise we open ourselves up to all kind of insane arbitrage attacks
    // since uxd *must* be fungible we cannot maintain accounts for individuals
    //
    // oook so... mint has to go like. for a particular depository...
    // we accept redeemable, proxy transfer coin to us, move coin onto mango (deposit)
    // create an opposite position on mango (place perp order). and then give uxd to user
    // for now we take fro granted that all deposited coins have a corresponding perp
    // if we want to take more esoteric forms of capital we may need to swap on serum
    //
    // im not sure controller should create uxd... idk what if we redeploy to a new address?
    // we should have liek... a function new, to set up the controller with state and owner
    // and a function register depository to whitelist a depository address
    // and create the mango account and such

    /////// Instruction functions ///////

    // NEW
    // create controller state, create uxd (this could happen elsewhere later)
    pub fn new(ctx: Context<New>) -> ProgramResult {
        let accounts = ctx.accounts.to_account_infos();

        let state_ctr = Pubkey::find_program_address(&[STATE_SEED], ctx.program_id).1;
        let uxd_ctr = Pubkey::find_program_address(&[UXD_SEED], ctx.program_id).1;

        let uxd_seed: &[&[&[u8]]] = &[&[UXD_SEED, &[uxd_ctr]]];

        let ix = initialize_mint(
            &spl_token::ID,
            &ctx.accounts.uxd_mint.key(),
            &ctx.accounts.state.key(),
            None,
            MINT_DECIMAL,
        )?;
        invoke_signed(&ix, &accounts, uxd_seed)?;

        ctx.accounts.state.authority_key = *ctx.accounts.authority.key;
        ctx.accounts.state.uxd_mint_key = *ctx.accounts.uxd_mint.key;

        Ok(())
    }

    // REGISTER DEPOSITORY
    // owner must be a signer and match owner in our state
    // create a mango account for the coin, create an entry indicating we created and trust this depository
    // create a proxy account. we need this because the owner (our state account) might sign for mango deposit
    // and the owner of the mango account and the token account must be the same
    // and we cant make the depository own the mango account because we need to sign for these accounts
    pub fn register_depository(ctx: Context<RegisterDepository>) -> ProgramResult {
        let accounts = ctx.accounts.to_account_infos();
        let coin_mint_key = ctx.accounts.coin_mint.key();

        let record_ctr = Pubkey::find_program_address(&[
            RECORD_SEED,
            ctx.accounts.depository.key.as_ref(),
        ], ctx.program_id).1;

        let passthrough_ctr = Pubkey::find_program_address(&[
            PASSTHROUGH_SEED,
            coin_mint_key.as_ref()
        ], ctx.program_id).1;

        let record_seed: &[&[&[u8]]] = &[&[RECORD_SEED, ctx.accounts.depository.key.as_ref(), &[record_ctr]]];
        let passthrough_seed: &[&[&[u8]]] = &[&[PASSTHROUGH_SEED, coin_mint_key.as_ref(), &[passthrough_ctr]]];

        let ix = initialize_account(
            &spl_token::ID,
            &ctx.accounts.coin_passthrough.key(),
            &coin_mint_key,
            &ctx.accounts.depository_record.key(),
        )?;
        invoke_signed(&ix, &accounts, passthrough_seed)?;





       // TODO this has to take an address of a depository... program? state object?
       // and it creates a mango account tied to the depository mint
       // for now the existence of that account is sufficient to prove it has been registered....
       // we odnt actually need to receive funds on the controller, just get approval and transfer, i think??
/*
        let accounts = ctx.accounts.to_account_infos();

        // create proxy account
        let (proxy_addr, proxy_ctr) = Pubkey::find_program_address(&[PROXY_SEED], ctx.program_id);
        let proxy_seed: &[&[&[u8]]] = &[&[PROXY_SEED, &[proxy_ctr]]];
        // let proxy_rent = ctx.accounts.rent.minimum_balance(ACCOUNT_SPAN as usize);
        // let proxy_init = create_account(ctx.accounts.owner.key, &proxy_addr, proxy_rent, ACCOUNT_SPAN, ctx.accounts.token_program.key);
        // invoke_signed(&proxy_init, &accounts, proxy_seed)?;
        // let depository_coin_key = accounts.coin_mint.key();
        //initialize proxy account
        let proxy_init = initialize_account(
            &spl_token::ID,
            &proxy_addr,
            ctx.accounts.coin_mint.to_account_info().key,
            ctx.accounts.state.to_account_info().key,
        )?;
        invoke_signed(&proxy_init, &accounts, &proxy_seed)?;

        // initialize mango or equivalent user account
        // using mango for now as a built in but later make different providers as separate internal functions
        // called based on a config

        // Accounts expected by this instruction (4):
        //
        // 0. `[]` mango_group_ai - MangoGroup that this mango account is for
        // 1. `[writable]` mango_account_ai - the mango account data
        // 2. `[signer]` owner_ai - Solana account of owner of the mango account
        // 3. `[]` rent_ai - Rent sysvar account
        let mango_cpi_program = ctx.accounts.mango_program.clone();
        let mango_cpi_accts = InitMangoAccount {
            mango_group: ctx.accounts.mango_group.to_account_info(),
            mango_account: ctx.accounts.mango_account.clone().into(),
            owner_account: ctx.accounts.proxy_account.clone().into(),
            rent: ctx.accounts.rent.clone(),
        };
        let mango_cpi_ctx = CpiContext::new(mango_cpi_program, mango_cpi_accts);
        mango_tester::cpi::init_mango_account(mango_cpi_ctx);
*/

        Ok(())
    }


    // pub fn mint(ctx: Context<Mint>) -> ProgramResult {
    //     // accept depository redeemable token
    //     // validate user input
    //     // call depository proxytransfer or equivalent
    //     // with reciever being controller proxy address (or controller mango account)
    //     // transfer to controller's Mango user account
    //     // call calc_swap_position
    //     // CPI call into mango to sell swaps according to ^
    //     // mint tokens from uxd_mint to user account
    // }
    //
    // pub fn redeem(ctx: Context<Redeem>) -> ProgramResult {
    //     // validate user input
    //     // burn user uxd
    //     // exchange depository redeemable token
    // }
    //
    //
    // fn proxy_transfer(ctx: Context<Proxy_transfer>) -> ProgramResult {
    //     // called by mint or rebalance
    //     // takes arguments for depository, amount, and target (if multiple targets)
    //     // depository has a fixed and short list of acceptable proxy transfer targets
    //     // CPI call into the depository contract
    //     // validate funds receipt and depository redeemable burn (on depository side)
    // }
    //
    // pub fn rebalance(ctx: Context<Rebalance>) -> ProgramResult {
    //     // validate caller is in rebalance signer(s)
    //     // WARNING DIFFICULT LOGIC
    //     // rebalance needs borrow/lending rate, outstanding pnl balance in an array across collateral types
    //     // probably better for it to just call by depository/collateral type for now,
    //     // since we're going for the single collateral version first
    //     // estimates rebalance cost eg transaction fees
    //     // uses some settable estimation constant (e?) for what the timescale to consider
    //     // if borrow * e * net pnl > est rebalance cost then rebal should go ahead
    //     // rebal for single collateral just amounts to settling some or all of the pnl and rehedging
    //     // for multi collateral there are two versions,
    //     // 1. that single collat balances in parallel for n depositories
    //         // could be a public function
    //     // 2. that optimizes for market rates across range of collateral types
    //         // will change portfolio balances in order to get the highest return on the basis trade
    //         // weighted array of parameters like liquidity, mkt cap, stability
    //         // Not a priority
    //
    // }
    //
    // /////// internal functions ///////
    //
    // fn calc_swap_position(collateral: Pubkey, amount: u64) -> ProgramResult {
    //     // used by mint and reblance to handle the calculation of swap positions
    //     // get collateral  oracle price from pyth (fn sig does nto reflect currently)
    //     // get swap pricing from mango api
    //     // price swaps/spot and then apply the requested amount
    // }

}

#[derive(Accounts)]
pub struct New<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [STATE_SEED],
        bump = Pubkey::find_program_address(&[STATE_SEED], program_id).1,
        payer = authority,
    )]
    pub state: ProgramAccount<'info, State>,
    #[account(
        init,
        seeds = [UXD_SEED],
        bump = Pubkey::find_program_address(&[UXD_SEED], program_id).1,
        payer = authority,
        owner = spl_token::ID,
        space = MINT_SPAN,
    )]
    pub uxd_mint: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(constraint = system_program.key() == system::ID)]
    pub system_program: AccountInfo<'info>,
    #[account(constraint = token_program.key() == spl_token::ID)]
    pub token_program: AccountInfo<'info>,
    #[account(constraint = program.key() == *program_id)]
    pub program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RegisterDepository<'info> {
    #[account(signer, mut, constraint = authority.key() == state.authority_key)]
    pub authority: AccountInfo<'info>,
    #[account(
        seeds = [STATE_SEED],
        bump = Pubkey::find_program_address(&[STATE_SEED], program_id).1,
    )]
    pub state: ProgramAccount<'info, State>,
    #[account(
        init,
        seeds = [RECORD_SEED, depository.key.as_ref()],
        bump = Pubkey::find_program_address(&[RECORD_SEED, depository.key.as_ref()], program_id).1,
        payer = authority,
    )]
    pub depository_record: ProgramAccount<'info, DepositoryRecord>,
    pub depository: AccountInfo<'info>,
    #[account(
        seeds = [depository::STATE_SEED],
        bump = Pubkey::find_program_address(&[depository::STATE_SEED], &depository.key()).1,
    )]
    pub depository_state: CpiAccount<'info, depository::State>,
    #[account(constraint = coin_mint.key() == depository_state.coin_mint_key)]
    pub coin_mint: CpiAccount<'info, Mint>,
    #[account(
        init,
        seeds = [PASSTHROUGH_SEED, coin_mint.key().as_ref()],
        bump = Pubkey::find_program_address(&[PASSTHROUGH_SEED, coin_mint.key().as_ref()], program_id).1,
        payer = authority,
        owner = spl_token::ID,
        space = ACCOUNT_SPAN,
    )]
    pub coin_passthrough: AccountInfo<'info>,
    //pub mango_group: CpiAccount<'info, MangoTester>,
    //pub mango_account: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    //pub mango_program: AccountInfo<'info>,
    pub program: AccountInfo<'info>,
}

#[account]
#[derive(Default)]
pub struct State {
    authority_key: Pubkey,
    uxd_mint_key: Pubkey,
}

#[account]
#[derive(Default)]
pub struct DepositoryRecord {
    state_key: Pubkey,
}
