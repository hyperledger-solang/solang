// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, IntoVal, Val};

const TOKEN_SRC: &str = r#"
contract token {
    mapping(address => int128) public balances;

    function mint(address to, int128 amount) public {
        require(amount >= 0, "amount must be non-negative");
        balances[to] = balances[to] + amount;
    }

    function transfer(address from, address to, int128 amount) public {
        require(amount >= 0, "amount must be non-negative");
        require(balances[from] >= amount, "insufficient balance");

        balances[from] = balances[from] - amount;
        balances[to] = balances[to] + amount;
    }

    function balance(address owner) public view returns (int128) {
        return balances[owner];
    }
}
"#;

const TIMELOCK_SRC: &str = r#"
contract claimable_balance {
    enum TimeBoundKind {
        Before,
        After
    }

    enum BalanceState {
        Uninitialized,
        Funded,
        Claimed
    }

    BalanceState public state;
    TimeBoundKind mode;
    int128 public amount;
    uint64 public bound_timestamp;

    function deposit(
        address from,
        address token_,
        int128 amount_,
        TimeBoundKind mode_,
        uint64 bound_timestamp_
    ) public {
        require(
            state == BalanceState.Uninitialized,
            "contract has been already initialized"
        );

        from.requireAuth();

        amount = amount_;
        mode = mode_;
        bound_timestamp = bound_timestamp_;

        address contract_address = address(this);
        bytes payload = abi.encode("transfer", from, contract_address, amount_);
        (bool success, bytes memory returndata) = token_.call(payload);
        success;
        returndata;

        state = BalanceState.Funded;
    }

    function claim(address token_, address claimant) public {
        claimant.requireAuth();

        require(state == BalanceState.Funded, "balance is not claimable");
        require(check_time_bound(), "time predicate is not fulfilled");

        state = BalanceState.Claimed;

        address contract_address = address(this);
        bytes memory payload = abi.encode("transfer", contract_address, claimant, amount);
        (bool success, bytes memory returndata) = token_.call(payload);
        success;
        returndata;
    }

    function check_time_bound() internal view returns (bool) {
        if (mode == TimeBoundKind.After) {
            return block.timestamp >= bound_timestamp;
        }

        return block.timestamp <= bound_timestamp;
    }
}
"#;

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
    let actual =
        runtime.invoke_contract(token, "balance", vec![owner.clone().into_val(&runtime.env)]);
    let expected: Val = expected.into_val(&runtime.env);
    assert!(expected.shallow_eq(&actual));
}

#[test]
fn timelock_after_rejects_early_and_allows_at_boundary() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();
    runtime.env.ledger().set_timestamp(900);

    let token = runtime.deploy_contract(TOKEN_SRC);
    let timelock = runtime.deploy_contract(TIMELOCK_SRC);

    let from = Address::generate(&runtime.env);
    let claimant = Address::generate(&runtime.env);

    mint(&runtime, &token, &from, 1_000);

    runtime.invoke_contract(
        &timelock,
        "deposit",
        vec![
            from.clone().into_val(&runtime.env),
            token.clone().into_val(&runtime.env),
            300_i128.into_val(&runtime.env),
            1_u32.into_val(&runtime.env),
            1_000_u64.into_val(&runtime.env),
        ],
    );

    assert_balance(&runtime, &token, &from, 700);
    assert_balance(&runtime, &token, &timelock, 300);

    runtime.env.ledger().set_timestamp(999);
    let logs = runtime.invoke_contract_expect_error(
        &timelock,
        "claim",
        vec![
            token.clone().into_val(&runtime.env),
            claimant.clone().into_val(&runtime.env),
        ],
    );

    assert!(logs
        .iter()
        .any(|entry| entry.contains("require condition failed")));

    assert_balance(&runtime, &token, &claimant, 0);
    assert_balance(&runtime, &token, &timelock, 300);

    runtime.env.ledger().set_timestamp(1_000);
    runtime.invoke_contract(
        &timelock,
        "claim",
        vec![
            token.clone().into_val(&runtime.env),
            claimant.clone().into_val(&runtime.env),
        ],
    );

    assert_balance(&runtime, &token, &claimant, 300);
    assert_balance(&runtime, &token, &timelock, 0);
}

#[test]
fn timelock_before_rejects_once_expired() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();
    runtime.env.ledger().set_timestamp(1_200);

    let token = runtime.deploy_contract(TOKEN_SRC);
    let timelock = runtime.deploy_contract(TIMELOCK_SRC);

    let from = Address::generate(&runtime.env);
    let claimant = Address::generate(&runtime.env);

    mint(&runtime, &token, &from, 500);

    runtime.invoke_contract(
        &timelock,
        "deposit",
        vec![
            from.clone().into_val(&runtime.env),
            token.clone().into_val(&runtime.env),
            200_i128.into_val(&runtime.env),
            0_u32.into_val(&runtime.env),
            1_250_u64.into_val(&runtime.env),
        ],
    );

    runtime.env.ledger().set_timestamp(1_251);
    let logs = runtime.invoke_contract_expect_error(
        &timelock,
        "claim",
        vec![
            token.clone().into_val(&runtime.env),
            claimant.clone().into_val(&runtime.env),
        ],
    );

    assert!(logs
        .iter()
        .any(|entry| entry.contains("require condition failed")));

    assert_balance(&runtime, &token, &from, 300);
    assert_balance(&runtime, &token, &claimant, 0);
    assert_balance(&runtime, &token, &timelock, 200);
}
