// SPDX-License-Identifier: Apache-2.0

use crate::{build_wasm, load_abi};
use ink_env::{
    hash::{Blake2x256, CryptoHash},
    topics::PrefixedValue,
};
use ink_metadata::InkProject;
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

    let solidity_abi = load_abi(&build_wasm(src, false, false)[0].1).spec();
    let ink_str = std::fs::read_to_string("testdata/ink/mother.json").unwrap();
    let ink_abi: InkProject = serde_json::from_str(&ink_str).unwrap();
}
