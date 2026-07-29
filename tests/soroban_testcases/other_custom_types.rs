// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/other_custom_types

use crate::build_solidity;
use soroban_sdk::{contracttype, FromVal, IntoVal};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub rank: u32,
    pub suit: u32,
}

const CONTRACT: &str = r#"
    contract other_custom_types {
        enum Suit { Hearts, Diamonds, Clubs, Spades }

        struct Card {
            uint32 rank;
            Suit suit;
        }

        Card stored;

        function set(Card memory c) public {
            stored = c;
        }

        function get() public view returns (Card memory) {
            return stored;
        }

        function pick(Suit s) public pure returns (Suit) {
            return s;
        }
    }
"#;

#[test]
fn enum_round_trips_through_abi() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // Suit.Clubs == 2
    let ret: u32 = FromVal::from_val(
        env,
        &runtime.invoke_contract(addr, "pick", vec![2_u32.into_val(env)]),
    );
    assert_eq!(ret, 2);
}

#[test]
fn struct_with_enum_field_round_trips() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // rank 7, Suit.Spades (== 3)
    let card = Card { rank: 7, suit: 3 };
    runtime.invoke_contract(addr, "set", vec![card.clone().into_val(env)]);

    let got = Card::from_val(env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(got, Card { rank: 7, suit: 3 });
}
