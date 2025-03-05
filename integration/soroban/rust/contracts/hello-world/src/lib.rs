#![no_std]
use soroban_sdk::{contract, contractimpl, Env, log};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn add(env: Env, a: u64, b: u64, c: u64) -> u64 {
        log!(&env,"Soroban SDK add function called!");
        a + b + c
    }
}
