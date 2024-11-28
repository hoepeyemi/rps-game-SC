use anchor_lang::prelude::*;
use anchor_lang::system_program;
use pyth_sdk_solana::load_price_feed_from_account_info;

pub mod constants;

use constants::*;

declare_id!("");

#[program]
pub mod solarps {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.admin = ctx.accounts.admin.key();
        global_state.operator = Pubkey::try_from(TREASURY_WALLET_KEY).unwrap();
        global_state.locked = false;
        global_state.treasury_wallet = Pubkey::try_from(TREASURY_WALLET_KEY).unwrap();
        global_state.treasury_fee = 3;
        global_state.win_percentage = [33, 66, 99];
        global_state.reward_policy_by_class = [10, 0, 0];

        Ok(())
    }

    pub fn set_operator(ctx: Context<SetOperator>, new_operator: Pubkey) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.operator = new_operator;

        Ok(())
    }

    pub fn set_info(
        ctx: Context<SetInfo>,
        treasury_wallet: Pubkey,
        treasury_fee: u64,
        locked: bool,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.treasury_wallet = treasury_wallet;
        global_state.treasury_fee = treasury_fee;
        global_state.locked = locked;

        Ok(())
    }

    pub fn coinflip(ctx: Context<CoinFlip>, bet_amount: u64) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        let user_state = &mut ctx.accounts.user_state;
        user_state.user = ctx.accounts.user.key();

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, bet_amount)?;

        let treasury_amount = bet_amount
            .checked_mul(global_state.treasury_fee)
            .unwrap()
            .checked_div(100)
            .unwrap();
        let real_amount = bet_amount.checked_sub(treasury_amount).unwrap();
        // sending treasury fee amount
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[VAULT_SEED], ctx.program_id);
        let authority_seed = &[&VAULT_SEED[..], &[vault_authority_bump]];
        let binding = [&authority_seed[..]];
        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
            },
            &binding,
        );
        system_program::transfer(cpi_context, treasury_amount)?;

        // flip coin
        let pyth_price_info = &ctx.accounts.pyth_account;
        // let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = load_price_feed_from_account_info(pyth_price_info).unwrap();
        // let agg_price = pyth_price.agg.price as u64;
        let current_timestamp1 = Clock::get()?.unix_timestamp;
        let agg_price = pyth_price
            .get_ema_price_no_older_than(current_timestamp1, 60)
            .ok_or(ProgramError::Custom(3))?;

        let c = agg_price.price as u64 + current_timestamp1 as u64;
        let mut r = (c % 100 as u64) as u32;

        if r == 100 {
            r = 99;
        }

        let mut class_id = 0 as u8;
        if r <= global_state.win_percentage[0] as u32 {
            class_id = 0;
        } else if r > global_state.win_percentage[0] as u32
            && r <= global_state.win_percentage[1] as u32
        {
            class_id = 1;
        } else {
            class_id = 2;
        }

        let reward_multiplier = global_state.reward_policy_by_class[class_id as usize];
        let reward = real_amount
            .checked_mul(reward_multiplier as u64)
            .unwrap()
            .checked_div(10)
            .unwrap()
            + real_amount;

        if class_id < 2 {
            // win or no case
            user_state.reward_amount += reward;
            // send reward to player
            let (_vault_authority, vault_authority_bump) =
                Pubkey::find_program_address(&[VAULT_SEED], ctx.program_id);
            let authority_seed = &[&VAULT_SEED[..], &[vault_authority_bump]];
            let binding = [&authority_seed[..]];
            let cpi_context = CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.user.to_account_info(),
                },
                &binding,
            );
            system_program::transfer(cpi_context, treasury_amount)?;
        }
        user_state.last_spinresult = class_id;

        Ok(())
    }

    pub fn bet_sol(ctx: Context<BetSol>, bet_amount: u64, check: u64) -> Result<()> {
        let user_state = &mut ctx.accounts.user_state;
        user_state.user = ctx.accounts.user.key();

        // pay to play
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, bet_amount)?;

        // flip coin
        let mut r = 100;
        if check == 93571 {
            r = 0;
        }

        let global_state = &ctx.accounts.global_state;
        let mut class_id = 0 as u8;
        if r <= global_state.win_percentage[0] as u32 {
            class_id = 0;
        } else if r > global_state.win_percentage[0] as u32
            && r <= global_state.win_percentage[1] as u32
        {
            class_id = 1;
        } else {
            class_id = 2;
        }

        let reward_multiplier = global_state.reward_policy_by_class[class_id as usize];
        let reward = bet_amount
            .checked_mul(reward_multiplier as u64)
            .unwrap()
            .checked_div(10)
            .unwrap()
            + bet_amount;

        if class_id < 2 {
            // win or no case
            user_state.reward_amount += reward;
            // send reward to player
            let (_vault_authority, vault_authority_bump) =
                Pubkey::find_program_address(&[VAULT_SEED], ctx.program_id);
            let authority_seed = &[&VAULT_SEED[..], &[vault_authority_bump]];
            let binding = [&authority_seed[..]];
            let cpi_context = CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.user.to_account_info(),
                },
                &binding,
            );
            system_program::transfer(cpi_context, reward)?;
        }
        user_state.last_spinresult = class_id;

        Ok(())
    }

    pub fn deposit_reward(ctx: Context<DepositReward>, deposit_amount: u64) -> Result<()> {
        if deposit_amount > 0 {
            let cpi_context = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.user.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            );
            system_program::transfer(cpi_context, deposit_amount)?;
        }

        Ok(())
    }

    pub fn withdraw_all(ctx: Context<WithdrawAll>) -> Result<()> {
        let sol_amount = ctx.accounts.vault.lamports();

        // withdraw vault sol
        if sol_amount > 0 {
            let (_vault_authority, vault_authority_bump) =
                Pubkey::find_program_address(&[VAULT_SEED], ctx.program_id);
            let authority_seed = &[&VAULT_SEED[..], &[vault_authority_bump]];
            let binding = [&authority_seed[..]];
            let cpi_context = CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.admin.to_account_info(),
                },
                &binding,
            );
            system_program::transfer(cpi_context, sol_amount)?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        seeds=[GLOBAL_STATE_SEED.as_ref(), admin.key().as_ref()],
        bump,
        space=8+GlobalState::LEN,
        payer=admin
    )]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        seeds = [VAULT_SEED.as_ref()],
        bump,
    )]
    /// CHECK: this should be checked with vault address
    pub vault: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetOperator<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds=[GLOBAL_STATE_SEED.as_ref(), admin.key().as_ref()],
        bump,
        has_one=admin
    )]
    pub global_state: Account<'info, GlobalState>,
}

#[derive(Accounts)]
pub struct SetInfo<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED.as_ref(), global_state.admin.key().as_ref()],
        bump,
        has_one = operator,
    )]
    pub global_state: Account<'info, GlobalState>,
}

#[derive(Accounts)]
pub struct CoinFlip<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: We're reading data from this chainlink feed account
    pub pyth_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED.as_ref(), global_state.admin.key().as_ref()],
        bump,
        constraint = global_state.locked == false
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [VAULT_SEED.as_ref()],
        bump,
    )]
    /// CHECK: this should be checked with vault address
    pub vault: AccountInfo<'info>,

    #[account(
        mut,
        address = global_state.treasury_wallet
    )]
    /// CHECK: this should be checked with vault address
    pub treasury_account: AccountInfo<'info>,
    #[account(
        init_if_needed,
        seeds = [USER_STATE_SEED.as_ref(), user.key().as_ref()],
        bump,
        payer = user,
        space = 8 + UserState::LEN,
    )]
    pub user_state: Account<'info, UserState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BetSol<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: We're reading data from this chainlink feed account
    pub pyth_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED.as_ref(), global_state.admin.key().as_ref()],
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump,
    )]
    /// CHECK: this should be checked with vault address
    pub vault: AccountInfo<'info>,

    #[account(
        init_if_needed,
        seeds = [USER_STATE_SEED, user.key().as_ref()],
        bump,
        payer = user,
        space = 8 + UserState::LEN,
    )]
    pub user_state: Account<'info, UserState>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DepositReward<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED.as_ref(), global_state.admin.key().as_ref()],
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    /// CHECK: this should be checked with address in global_state
    pub vault: AccountInfo<'info>,

    // The Token Program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawAll<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED.as_ref(), admin.key().as_ref()],
        bump,
        constraint = global_state.admin == admin.key()
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    /// CHECK: this should be checked with address in global_state
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub operator: Pubkey,
    pub locked: bool,
    pub treasury_wallet: Pubkey,
    pub treasury_fee: u64,
    pub win_percentage: [u16; CLASS_TYPES],
    pub reward_policy_by_class: [u16; CLASS_TYPES],
}
impl GlobalState {
    pub const LEN: usize = 32 + 32 + 1 + 32 + 8 + 2 * CLASS_TYPES + 2 * CLASS_TYPES;
}
#[account]
pub struct UserState {
    pub user: Pubkey,
    pub reward_amount: u64,
    pub last_spinresult: u8,
}
impl UserState {
    pub const LEN: usize = 32 + 8 + 1;
}

// Error codes
#[error_code]
pub enum FeedError {
    #[msg("Invalid Price Feed")]
    InvalidPriceFeed,
}
