// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, SorobanEnv};
use soroban_sdk::{contracttype, testutils::Address as _, Address, IntoVal, Val};

#[contracttype]
#[derive(Clone)]
struct Item {
    account: Address,
    amount: u64,
    min_recv: u64,
}

fn item(env: &soroban_sdk::Env, amount: u64, min_recv: u64) -> Item {
    Item {
        account: Address::generate(env),
        amount,
        min_recv,
    }
}

#[test]
fn struct_array_member_reads() {
    let runtime = build_solidity(
        r#"
        contract t {
            struct Item {
                address account;
                uint64 amount;
                uint64 min_recv;
            }

            function sum_amounts(Item[] memory items) public pure returns (uint64) {
                uint64 total = 0;
                for (uint64 i = 0; i < items.length; i++) {
                    total = total + items[i].amount;
                }
                return total;
            }

            function nth_account(Item[] memory items, uint64 n) public pure returns (address) {
                return items[n].account;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let first = item(env, 10, 1);
    let second = item(env, 20, 2);
    let third = item(env, 30, 3);
    let items = soroban_sdk::vec![env, first.clone(), second.clone(), third.clone()];

    // sum_amounts([10, 20, 30]) => 60
    let ret = runtime.invoke_contract(addr, "sum_amounts", vec![items.clone().into_val(env)]);
    let expected: Val = 60_u64.into_val(env);
    assert!(expected.shallow_eq(&ret));

    // nth_account(items, 1) => second.account
    let ret = runtime.invoke_contract(
        addr,
        "nth_account",
        vec![items.into_val(env), 1_u64.into_val(env)],
    );
    let expected: Val = second.account.into_val(env);
    assert!(expected.shallow_eq(&ret));
}

const SWAP_STUB_SRC: &str = r#"
contract swap_stub {
    uint64 public swaps;

    function swap(
        address a,
        address b,
        address token_a,
        address token_b,
        uint64 amount_a,
        uint64 min_a_for_b,
        uint64 amount_b,
        uint64 min_b_for_a
    ) public {
        require(amount_a >= min_b_for_a && amount_b >= min_a_for_b, "amounts do not match");
        swaps = swaps + 1;
    }
}
"#;

// Kept in sync with examples/soroban/atomic_multiswap/atomic_multiswap.sol.
const MULTISWAP_SRC: &str = r#"
contract atomic_multiswap {
    struct SwapSpec {
        address account;
        uint64 amount;
        uint64 min_recv;
    }

    function multi_swap(
        address swap_contract,
        address token_a,
        address token_b,
        SwapSpec[] memory a,
        SwapSpec[] memory b
    ) public {
        bool[] memory matched = new bool[](b.length);

        for (uint64 i = 0; i < a.length; i++) {
            SwapSpec memory acc_a = a[i];

            for (uint64 j = 0; j < b.length; j++) {
                if (matched[j]) {
                    continue;
                }

                SwapSpec memory acc_b = b[j];

                if (acc_a.amount >= acc_b.min_recv && acc_b.amount >= acc_a.min_recv) {
                    bytes payload = abi.encode(
                        "swap",
                        acc_a.account,
                        acc_b.account,
                        token_a,
                        token_b,
                        acc_a.amount,
                        acc_a.min_recv,
                        acc_b.amount,
                        acc_b.min_recv
                    );

                    (bool success, bytes returndata) = swap_contract.call(payload);

                    if (success) {
                        matched[j] = true;
                        break;
                    }
                }
            }
        }
    }
}
"#;

#[test]
fn multi_swap_matches_pairs() {
    let mut runtime = SorobanEnv::new();

    let swap_stub = runtime.deploy_contract(SWAP_STUB_SRC);
    let multiswap = runtime.deploy_contract(MULTISWAP_SRC);

    runtime.env.mock_all_auths();

    let env = runtime.env.clone();
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    // a[0] matches b[0] (100 >= 85 and 95 >= 90); a[1] then matches b[1] (50 >= 40 and
    // 46 >= 45); a[2] matches nothing (its min_recv of 1000 is unsatisfiable).
    let a = soroban_sdk::vec![
        &env,
        item(&env, 100, 90),
        item(&env, 50, 45),
        item(&env, 5, 1000),
    ];
    let b = soroban_sdk::vec![
        &env,
        item(&env, 95, 85),
        item(&env, 46, 40),
        item(&env, 10, 1),
    ];

    runtime.invoke_contract(
        &multiswap,
        "multi_swap",
        vec![
            swap_stub.clone().into_val(&env),
            token_a.into_val(&env),
            token_b.into_val(&env),
            a.into_val(&env),
            b.into_val(&env),
        ],
    );

    let swaps = runtime.invoke_contract(&swap_stub, "swaps", vec![]);
    let expected: Val = 2_u64.into_val(&env);
    assert!(expected.shallow_eq(&swaps));
}
