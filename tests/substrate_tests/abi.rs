// SPDX-License-Identifier: Apache-2.0

use crate::{build_wasm, load_abi};
use ink_env::{
    hash::{Blake2x256, CryptoHash},
    topics::PrefixedValue,
};
use ink_metadata::{InkProject, TypeSpec};
use ink_primitives::{AccountId, Hash};
use parity_scale_codec::Encode;
use scale_info::form::PortableForm;
use solang::{file_resolver::FileResolver, Target};
use std::ffi::OsStr;

#[test]
fn mother() {
    // Partial mock of the ink! "mother" integration test.
    let src = r##"
import "substrate";

contract Mother {
    enum Status {
        NotStarted,
        OpeningPeriod
    }

    struct Auction {
        string name;
        Hash subject;
        uint64[3] terms;
        Status status;
        bool finalized;
        bytes vector;
    }

    Auction auction;
    mapping(address => uint128) balances;

    function echo_auction(Auction _auction) public pure returns (Auction) {
        return _auction;
    }
}"##;

    let solang_abi = load_abi(&build_wasm(src, false, false)[0].1);
    let ink_str = std::fs::read_to_string("testdata/ink/mother.json").unwrap();
    let ink_abi: InkProject = serde_json::from_str(&ink_str).unwrap();

    let solang_env = solang_abi.spec().environment();
    let ink_env = ink_abi.spec().environment();

    assert_path(solang_env.timestamp(), ink_env.timestamp());
    assert_path(solang_env.account_id(), ink_env.account_id());
    assert_path(solang_env.hash(), ink_env.hash());
    assert_path(solang_env.balance(), ink_env.balance());
    assert_path(solang_env.block_number(), ink_env.block_number());
    assert_eq!(solang_env.max_event_topics(), ink_env.max_event_topics());
}

fn assert_path(a: &TypeSpec<PortableForm>, b: &TypeSpec<PortableForm>) {
    assert_eq!(a.display_name(), b.display_name());
}
