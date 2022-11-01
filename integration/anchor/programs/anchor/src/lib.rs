// SPDX-License-Identifier: Apache-2.0

use anchor_lang::prelude::*;

declare_id!("z7FbDfQDfucxJz5o8jrGLgvSbdoeSqX5VrxBb5TVjHq");

/// Anchor example for testing with Solang.
/// This doc comment exist for testing metadata doc comments
#[program]
pub mod anchor {
    use super::*;

    // a few primitives
    pub fn initialize(
        ctx: Context<Initialize>,
        data1: bool,
        data2: i32,
        data3: u64,
        data4: Pubkey,
    ) -> Result<()> {
        let my_accounts = &mut ctx.accounts.my_account;
        my_accounts.data1 = data1;
        my_accounts.data2 = data2;
        my_accounts.data3 = data3;
        my_accounts.data4 = data4;
        Ok(())
    }

    /// string test
    pub fn strings(_ctx: Context<NoAccountsNeeded>, input: String, data: u8) -> Result<String> {
        Ok(format!("input:{} data:{}", input, data))
    }

    /// bytes test
    pub fn bytes(_ctx: Context<NoAccountsNeeded>, input: Vec<u8>, data: u64) -> Result<Vec<u8>> {
        let mut input = input;
        input[data as usize] = !input[data as usize];
        Ok(input)
    }

    /// Sum all fields of input and start
    pub fn sum(_ctx: Context<NoAccountsNeeded>, input: Vec<u64>, start: u64) -> Result<u64> {
        Ok(input.iter().fold(start, |acc, x| acc + x))
    }

    pub fn sector001(_ctx: Context<NoAccountsNeeded>) -> Result<Sector> {
        Ok(Sector {
            suns: 1,
            mclass: vec![Planet::Earth],
            calldata: *b"0123456789012",
        })
    }

    pub fn has_planet(
        _ctx: Context<NoAccountsNeeded>,
        sector: Sector,
        planet: Planet,
    ) -> Result<bool> {
        Ok(sector.mclass.contains(&planet))
    }

    pub fn states(ctx: Context<State>) -> Result<returns> {
        let my_account = &ctx.accounts.my_account;

        Ok(returns {
            default: my_account.data1,
            delete: my_account.data2,
            fallback: my_account.data3,
            assembly: my_account.data4,
        })
    }

    pub fn multi_dimensional(
        _ctx: Context<NoAccountsNeeded>,
        arr: [[u16; 3]; 4],
    ) -> Result<[[u16; 4]; 3]> {
        let mut res = [[0u16; 4]; 3];

        #[allow(clippy::needless_range_loop)]
        for x in 0..4 {
            for y in 0..3 {
                res[y][x] = arr[x][y];
            }
        }

        Ok(res)
    }
}

/// Planets
/// in our
/// Solar System
#[allow(non_camel_case_types)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub enum Planet {
    Mercury,
    Venus,
    /// 3rd rock from the sun
    Earth,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    /// Solidity keyword!
    anonymous,
}

/// Can a sector have multiple
/// solar systems?
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Sector {
    /// A binary system can have multiple suns
    suns: u64,
    /// Which planets can support life?
    mclass: Vec<Planet>,
    /// Just
    /// so
    /// Random field with solidity keyword
    calldata: [u8; 13],
}

/// Anchor requires that multiple return values must be put into a struct
#[allow(non_camel_case_types)]
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct returns {
    default: bool,
    delete: i32,
    fallback: u64,
    assembly: Pubkey,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 64)]
    pub my_account: Account<'info, MyAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct NoAccountsNeeded {}

#[account]
pub struct MyAccount {
    pub data1: bool,
    pub data2: i32,
    pub data3: u64,
    pub data4: Pubkey,
}

#[derive(Accounts)]
pub struct State<'info> {
    #[account()]
    pub my_account: Account<'info, MyAccount>,
}
