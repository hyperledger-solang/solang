// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use soroban_sdk::{testutils::Address as _, Address, IntoVal, Val};

const TOKEN_SRC: &str = r#"
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

    function mint(address to, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        admin.requireAuth();
        balances[to] = balances[to] + amount;
    }

    function transfer(address from, address to, int128 amount) public {
        require(amount >= 0, "Amount must be non-negative");
        from.requireAuth();
        require(balances[from] >= amount, "Insufficient balance");
        balances[from] = balances[from] - amount;
        balances[to] = balances[to] + amount;
    }

    function balance(address addr) public view returns (int128) {
        return balances[addr];
    }
}
"#;

const ATOMIC_SWAP_SRC: &str = r#"
contract atomic_swap {
    function swap(
        address a,
        address b,
        address token_a,
        address token_b,
        int128 amount_a,
        int128 min_b_for_a,
        int128 amount_b,
        int128 min_a_for_b
    ) public {
        require(amount_b >= min_b_for_a, "not enough token B for token A");
        require(amount_a >= min_a_for_b, "not enough token A for token B");

        a.requireAuth();
        b.requireAuth();

        move_token(token_a, a, b, amount_a, min_a_for_b);
        move_token(token_b, b, a, amount_b, min_b_for_a);
    }

    function move_token(
        address token,
        address from,
        address to,
        int128 max_spend_amount,
        int128 transfer_amount
    ) internal {
        address contract_address = address(this);

        bytes payload = abi.encode("transfer", from, contract_address, max_spend_amount);
        (bool success, bytes returndata) = token.call(payload);

        payload = abi.encode("transfer", contract_address, to, transfer_amount);
        (success, returndata) = token.call(payload);

        payload = abi.encode(
            "transfer",
            contract_address,
            from,
            max_spend_amount - transfer_amount
        );
        (success, returndata) = token.call(payload);
    }
}
"#;

fn deploy_token(runtime: &mut SorobanEnv, name: &str, symbol: &str) -> Address {
    let admin = Address::generate(&runtime.env);
    let decimals: Val = 18_u32.into_val(&runtime.env);
    let name = soroban_sdk::String::from_str(&runtime.env, name);
    let symbol = soroban_sdk::String::from_str(&runtime.env, symbol);

    runtime.deploy_contract_with_args(TOKEN_SRC, (admin, name, symbol, decimals))
}

fn mint(runtime: &SorobanEnv, token: &Address, to: &Address, amount: i128) {
    runtime.invoke_contract(
        token,
        "mint",
        vec![
            to.clone().into_val(&runtime.env),
            amount.into_val(&runtime.env),
        ],
    );
}

fn assert_balance(runtime: &SorobanEnv, token: &Address, owner: &Address, expected: i128) {
    let balance =
        runtime.invoke_contract(token, "balance", vec![owner.clone().into_val(&runtime.env)]);
    let expected: Val = expected.into_val(&runtime.env);

    assert!(expected.shallow_eq(&balance));
}

#[test]
fn atomic_swap_end_to_end_test() {
    let mut runtime = SorobanEnv::new();

    let token_a = deploy_token(&mut runtime, "Token A", "TKA");
    let token_b = deploy_token(&mut runtime, "Token B", "TKB");
    let swap = runtime.deploy_contract(ATOMIC_SWAP_SRC);

    runtime.env.mock_all_auths();

    let a = Address::generate(&runtime.env);
    let b = Address::generate(&runtime.env);

    mint(&runtime, &token_a, &a, 100);
    mint(&runtime, &token_b, &b, 80);

    runtime.invoke_contract(
        &swap,
        "swap",
        vec![
            a.clone().into_val(&runtime.env),
            b.clone().into_val(&runtime.env),
            token_a.clone().into_val(&runtime.env),
            token_b.clone().into_val(&runtime.env),
            40_i128.into_val(&runtime.env),
            30_i128.into_val(&runtime.env),
            50_i128.into_val(&runtime.env),
            35_i128.into_val(&runtime.env),
        ],
    );

    assert_balance(&runtime, &token_a, &a, 65);
    assert_balance(&runtime, &token_a, &b, 35);
    assert_balance(&runtime, &token_a, &swap, 0);

    assert_balance(&runtime, &token_b, &a, 30);
    assert_balance(&runtime, &token_b, &b, 50);
    assert_balance(&runtime, &token_b, &swap, 0);
}

#[test]
fn atomic_swap_rejects_when_min_price_not_met() {
    let mut runtime = SorobanEnv::new();

    let token_a = deploy_token(&mut runtime, "Token A", "TKA");
    let token_b = deploy_token(&mut runtime, "Token B", "TKB");
    let swap = runtime.deploy_contract(ATOMIC_SWAP_SRC);

    runtime.env.mock_all_auths();

    let a = Address::generate(&runtime.env);
    let b = Address::generate(&runtime.env);

    mint(&runtime, &token_a, &a, 100);
    mint(&runtime, &token_b, &b, 80);

    let logs = runtime.invoke_contract_expect_error(
        &swap,
        "swap",
        vec![
            a.clone().into_val(&runtime.env),
            b.clone().into_val(&runtime.env),
            token_a.clone().into_val(&runtime.env),
            token_b.clone().into_val(&runtime.env),
            40_i128.into_val(&runtime.env),
            60_i128.into_val(&runtime.env),
            50_i128.into_val(&runtime.env),
            35_i128.into_val(&runtime.env),
        ],
    );

    assert!(logs
        .iter()
        .any(|entry| entry.contains("require condition failed")));

    assert_balance(&runtime, &token_a, &a, 100);
    assert_balance(&runtime, &token_a, &b, 0);
    assert_balance(&runtime, &token_a, &swap, 0);

    assert_balance(&runtime, &token_b, &a, 0);
    assert_balance(&runtime, &token_b, &b, 80);
    assert_balance(&runtime, &token_b, &swap, 0);
}
