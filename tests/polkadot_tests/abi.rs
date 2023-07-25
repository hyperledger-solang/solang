// SPDX-License-Identifier: Apache-2.0

use crate::{build_wasm, load_abi};
use ink_metadata::{InkProject, TypeSpec};
use once_cell::sync::Lazy;
use scale_info::form::PortableForm;
use std::sync::Mutex;

/// Partially mimicking the ink! "mother" integration test.
static MOTHER: Lazy<Mutex<(InkProject, InkProject)>> = Lazy::new(|| {
    let src = r#"
import "polkadot";

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
}"#;

    let solang_abi = load_abi(&build_wasm(src, false, false)[0].1);
    let ink_str = std::fs::read_to_string("testdata/ink/mother.json").unwrap();
    let ink_abi: InkProject = serde_json::from_str(&ink_str).unwrap();

    Mutex::new((solang_abi, ink_abi))
});

fn eq_display(a: &TypeSpec<PortableForm>, b: &TypeSpec<PortableForm>) {
    assert_eq!(a.display_name(), b.display_name());
}

#[test]
fn environment_matches_ink() {
    let mother = MOTHER.lock().unwrap();
    let (solang, ink) = (mother.0.spec().environment(), mother.1.spec().environment());

    eq_display(solang.timestamp(), ink.timestamp());
    eq_display(solang.account_id(), ink.account_id());
    eq_display(solang.hash(), ink.hash());
    eq_display(solang.balance(), ink.balance());
    eq_display(solang.block_number(), ink.block_number());
    assert_eq!(solang.max_event_topics(), ink.max_event_topics());
}

#[test]
fn address_type_path_exists() {
    let mother = MOTHER.lock().unwrap();
    let (solang, ink) = (mother.0.registry(), mother.1.registry());

    let ink_address = &ink.types[8].ty.path;
    assert!(solang.types.iter().any(|t| &t.ty.path == ink_address));
}

#[test]
fn hash_type_path_exists() {
    let mother = MOTHER.lock().unwrap();
    let (solang, ink) = (mother.0.registry(), mother.1.registry());

    let ink_hash = &ink.types[1].ty.path;
    assert!(solang.types.iter().any(|t| &t.ty.path == ink_hash));
}

#[test]
fn inherited_externally_callable_functions() {
    let src = r##"
    interface IERC165 {
        function supportsInterface(bytes4 interfaceId) external view returns (bool);
    }
    
    interface IERC1155 is IERC165 {}
    
    contract ERC165 is IERC165 {
        function supportsInterface(
            bytes4 interfaceId
        ) public view virtual override returns (bool) {}
    }
    
    contract ERC1155 is ERC165, IERC1155 {
        function supportsInterface(
            bytes4 interfaceId
        ) public view virtual override(ERC165, IERC165) returns (bool) {}
    }
    
    contract MyToken is ERC1155 {}
    "##;

    let abi = load_abi(&build_wasm(src, false, false)[0].1);
    let messages = abi.spec().messages();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].label(), "supportsInterface");
}
