// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use soroban_sdk::{testutils::Address as _, Address, IntoVal, Val};

#[test]
fn token_end_to_end_test() {
    let mut runtime = SorobanEnv::new();

    let admin = Address::generate(&runtime.env);
    let name = soroban_sdk::String::from_str(&runtime.env, "Test Token");
    let symbol = soroban_sdk::String::from_str(&runtime.env, "TTK");
    let decimals: Val = 18_u32.into_val(&runtime.env);

    let contract_src = r#"
        contract token {
            address public admin;
            uint32 public decimals;
            string public name;
            string public symbol;

            constructor(address _admin, string memory _name, string memory _symbol, uint32 _decimals) {
                admin = _admin;
                name = _name;
                symbol = _symbol;
                decimals = _decimals;
            }

            mapping(address => int128) public balances;
            mapping(address => mapping(address => int128)) public allowances;

            function mint(address to, int128 amount) public {
                require(amount >= 0, "Amount must be non-negative");
                admin.requireAuth();
                setBalance(to, balance(to) + amount);
            }

            function approve(address owner, address spender, int128 amount) public {
                require(amount >= 0, "Amount must be non-negative");
                owner.requireAuth();
                allowances[owner][spender] = amount;
            }

            function transfer(address from, address to, int128 amount) public {
                require(amount >= 0, "Amount must be non-negative");
                from.requireAuth();
                require(balance(from) >= amount, "Insufficient balance");
                setBalance(from, balance(from) - amount);
                setBalance(to, balance(to) + amount);
            }

            function transfer_from(address spender, address from, address to, int128 amount) public {
                require(amount >= 0, "Amount must be non-negative");
                spender.requireAuth();
                require(balance(from) >= amount, "Insufficient balance");
                require(allowance(from, spender) >= amount, "Insufficient allowance");
                setBalance(from, balance(from) - amount);
                setBalance(to, balance(to) + amount);
                allowances[from][spender] -= amount;
            }

            function burn(address from, int128 amount) public {
                require(amount >= 0, "Amount must be non-negative");
                require(balance(from) >= amount, "Insufficient balance");
                from.requireAuth();
                setBalance(from, balance(from) - amount);
            }

            function burn_from(address spender, address from, int128 amount) public {
                require(amount >= 0, "Amount must be non-negative");
                spender.requireAuth();
                require(balance(from) >= amount, "Insufficient balance");
                require(allowance(from, spender) >= amount, "Insufficient allowance");
                setBalance(from, balance(from) - amount);
                allowances[from][spender] -= amount;
            }

            function setBalance(address addr, int128 amount) internal {
                balances[addr] = amount;
            }

            function balance(address addr) public view returns (int128) {
                return balances[addr];
            }

            function allowance(address owner, address spender) public view returns (int128) {
                return allowances[owner][spender];
            }
        }
    "#;

    let addr = runtime.deploy_contract_with_args(contract_src, (admin, name, symbol, decimals));

    runtime.env.mock_all_auths();

    let user1 = Address::generate(&runtime.env);
    let user2 = Address::generate(&runtime.env);
    let user3 = Address::generate(&runtime.env);

    runtime.invoke_contract(
        &addr,
        "mint",
        vec![
            user1.clone().into_val(&runtime.env),
            100_i128.into_val(&runtime.env),
        ],
    );

    runtime.invoke_contract(
        &addr,
        "transfer",
        vec![
            user1.clone().into_val(&runtime.env),
            user2.clone().into_val(&runtime.env),
            25_i128.into_val(&runtime.env),
        ],
    );

    runtime.invoke_contract(
        &addr,
        "approve",
        vec![
            user1.clone().into_val(&runtime.env),
            user3.clone().into_val(&runtime.env),
            30_i128.into_val(&runtime.env),
        ],
    );

    runtime.invoke_contract(
        &addr,
        "transfer_from",
        vec![
            user3.clone().into_val(&runtime.env),
            user1.clone().into_val(&runtime.env),
            user3.clone().into_val(&runtime.env),
            10_i128.into_val(&runtime.env),
        ],
    );

    runtime.invoke_contract(
        &addr,
        "burn",
        vec![
            user2.clone().into_val(&runtime.env),
            5_i128.into_val(&runtime.env),
        ],
    );

    runtime.invoke_contract(
        &addr,
        "burn_from",
        vec![
            user3.clone().into_val(&runtime.env),
            user1.clone().into_val(&runtime.env),
            15_i128.into_val(&runtime.env),
        ],
    );

    let b1 = runtime.invoke_contract(&addr, "balance", vec![user1.into_val(&runtime.env)]);
    let b2 = runtime.invoke_contract(&addr, "balance", vec![user2.into_val(&runtime.env)]);
    let b3 = runtime.invoke_contract(&addr, "balance", vec![user3.into_val(&runtime.env)]);

    let expected1: Val = 50_i128.into_val(&runtime.env);
    let expected2: Val = 20_i128.into_val(&runtime.env);
    let expected3: Val = 10_i128.into_val(&runtime.env);

    assert!(expected1.shallow_eq(&b1));
    assert!(expected2.shallow_eq(&b2));
    assert!(expected3.shallow_eq(&b3));
}
