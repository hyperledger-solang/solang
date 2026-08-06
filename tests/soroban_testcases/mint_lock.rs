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

    function balance(address addr) public view returns (int128) {
        return balances[addr];
    }
}
"#;

// Mapping of https://github.com/stellar/soroban-examples/tree/main/mint-lock
//
// An admin authorizes minters with a per-minter cap. Each minter can mint tokens (through a
// cross-contract call into the token) up to its configured limit; minting past the limit reverts.
const MINT_LOCK_SRC: &str = r#"
contract mint_lock {
    address admin;
    mapping(address => int128) limit;
    mapping(address => int128) minted;

    function set_admin(address new_admin) public {
        admin = new_admin;
    }

    function set_limit(address minter, int128 max_amount) public {
        admin.requireAuth();
        limit[minter] = max_amount;
    }

    function mint(address token, address minter, address to, int128 amount) public {
        minter.requireAuth();
        require(amount > 0, "amount must be positive");
        require(minted[minter] + amount <= limit[minter], "over mint limit");

        minted[minter] = minted[minter] + amount;

        bytes payload = abi.encode("mint", to, amount);
        (bool ok, bytes returndata) = token.call(payload);
    }
}
"#;

fn deploy_token(runtime: &mut SorobanEnv, admin: &Address) -> Address {
    let decimals: Val = 18_u32.into_val(&runtime.env);
    let name = soroban_sdk::String::from_str(&runtime.env, "Token");
    let symbol = soroban_sdk::String::from_str(&runtime.env, "TKN");

    runtime.deploy_contract_with_args(TOKEN_SRC, (admin.clone(), name, symbol, decimals))
}

fn assert_balance(runtime: &SorobanEnv, token: &Address, owner: &Address, expected: i128) {
    let balance =
        runtime.invoke_contract(token, "balance", vec![owner.clone().into_val(&runtime.env)]);
    let expected: Val = expected.into_val(&runtime.env);
    assert!(expected.shallow_eq(&balance));
}

#[test]
fn mint_lock_allows_minting_up_to_limit() {
    let mut runtime = SorobanEnv::new();

    // The token's admin is the mint_lock contract itself, so mint_lock's cross-contract mint is
    // authorized automatically (a contract authorizes its own sub-invocations).
    let mint_lock = runtime.deploy_contract(MINT_LOCK_SRC);
    let token = deploy_token(&mut runtime, &mint_lock);

    runtime.env.mock_all_auths();

    let admin = Address::generate(&runtime.env);
    let minter = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    runtime.invoke_contract(&mint_lock, "set_admin", vec![admin.clone().into_val(&runtime.env)]);
    runtime.invoke_contract(
        &mint_lock,
        "set_limit",
        vec![
            minter.clone().into_val(&runtime.env),
            100_i128.into_val(&runtime.env),
        ],
    );

    // Two mints that together stay within the 100 limit.
    runtime.invoke_contract(
        &mint_lock,
        "mint",
        vec![
            token.clone().into_val(&runtime.env),
            minter.clone().into_val(&runtime.env),
            user.clone().into_val(&runtime.env),
            60_i128.into_val(&runtime.env),
        ],
    );
    runtime.invoke_contract(
        &mint_lock,
        "mint",
        vec![
            token.clone().into_val(&runtime.env),
            minter.clone().into_val(&runtime.env),
            user.clone().into_val(&runtime.env),
            40_i128.into_val(&runtime.env),
        ],
    );

    assert_balance(&runtime, &token, &user, 100);
}

#[test]
fn mint_lock_rejects_minting_over_limit() {
    let mut runtime = SorobanEnv::new();

    // The token's admin is the mint_lock contract itself, so mint_lock's cross-contract mint is
    // authorized automatically (a contract authorizes its own sub-invocations).
    let mint_lock = runtime.deploy_contract(MINT_LOCK_SRC);
    let token = deploy_token(&mut runtime, &mint_lock);

    runtime.env.mock_all_auths();

    let admin = Address::generate(&runtime.env);
    let minter = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    runtime.invoke_contract(&mint_lock, "set_admin", vec![admin.clone().into_val(&runtime.env)]);
    runtime.invoke_contract(
        &mint_lock,
        "set_limit",
        vec![
            minter.clone().into_val(&runtime.env),
            100_i128.into_val(&runtime.env),
        ],
    );

    runtime.invoke_contract(
        &mint_lock,
        "mint",
        vec![
            token.clone().into_val(&runtime.env),
            minter.clone().into_val(&runtime.env),
            user.clone().into_val(&runtime.env),
            60_i128.into_val(&runtime.env),
        ],
    );

    // 60 + 60 = 120 > 100 limit: rejected.
    let logs = runtime.invoke_contract_expect_error(
        &mint_lock,
        "mint",
        vec![
            token.clone().into_val(&runtime.env),
            minter.clone().into_val(&runtime.env),
            user.clone().into_val(&runtime.env),
            60_i128.into_val(&runtime.env),
        ],
    );

    assert!(logs
        .iter()
        .any(|entry| entry.contains("require condition failed")));

    // Only the first mint went through.
    assert_balance(&runtime, &token, &user, 60);
}
