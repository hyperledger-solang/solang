// SPDX-License-Identifier: Apache-2.0

use crate::{build_wasm, load_abi};
use ink_metadata::{InkProject, TypeSpec};
use once_cell::sync::Lazy;
use scale_info::{
    form::PortableForm, Path, TypeDef, TypeDefComposite, TypeDefPrimitive, TypeDefVariant,
};
use std::{collections::HashSet, sync::Mutex};

macro_rules! path {
    ($( $segments:expr ),*) => {
        Path::from_segments_unchecked([$($segments),*].iter().map(ToString::to_string))
    }
}

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

    let solang_abi = load_abi(&build_wasm(src, false)[0].1);
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

    let abi = load_abi(&build_wasm(src, false)[0].1);
    let messages = abi.spec().messages();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].label(), "supportsInterface");
}

/// Ensure that the correct selector and data type for Error(String) and
/// Panic(uint256) is present in the metadata.
#[test]
fn error_and_panic_in_lang_error() {
    let src = r##"
    contract Foo { uint public foo; }
    "##;
    let abi = load_abi(&build_wasm(src, false)[0].1);

    // Find them in lang_error
    let (error_ty_id, panic_ty_id) = match &abi
        .registry()
        .resolve(abi.spec().lang_error().ty().id)
        .unwrap()
        .type_def
    {
        TypeDef::<PortableForm>::Variant(TypeDefVariant::<PortableForm> { variants }) => {
            let error = variants.iter().find(|v| v.name == "Error").unwrap();
            let panic = variants.iter().find(|v| v.name == "Panic").unwrap();
            (error.fields[0].ty.id, panic.fields[0].ty.id)
        }
        _ => panic!("unexpected lang_err type def"),
    };

    // Asserts for Error
    let error_ty = abi.registry().resolve(error_ty_id).unwrap();
    let error_ty_id = match &error_ty.type_def {
        TypeDef::<PortableForm>::Composite(TypeDefComposite::<PortableForm> { fields }) => {
            assert_eq!(error_ty.path, path!("0x08c379a0"));
            fields[0].ty.id
        }
        _ => panic!("expected Error(string) type"),
    };
    let error_ty = abi.registry().resolve(error_ty_id).unwrap();
    match &error_ty.type_def {
        TypeDef::<PortableForm>::Primitive(TypeDefPrimitive::Str) => {
            assert_eq!(error_ty.path, path!("string"))
        }
        _ => panic!("expected Error(string) type"),
    };

    // Asserts for Panic
    let panic_ty = abi.registry().resolve(panic_ty_id).unwrap();
    let panic_ty_id = match &panic_ty.type_def {
        TypeDef::<PortableForm>::Composite(TypeDefComposite::<PortableForm> { fields }) => {
            assert_eq!(panic_ty.path, path!("0x4e487b71"));
            fields[0].ty.id
        }
        _ => panic!("expected Panic(uint256) type"),
    };
    let panic_ty = abi.registry().resolve(panic_ty_id).unwrap();
    match &panic_ty.type_def {
        TypeDef::<PortableForm>::Primitive(TypeDefPrimitive::U256) => {
            assert_eq!(panic_ty.path, path!("uint256"))
        }
        _ => panic!("expected Panic(uint256) type"),
    };
}

/// Ensure that custom errors end up correctly in the metadata.
#[test]
fn custom_errors_in_metadata() {
    let src = r#"
        struct Bar { uint foo; string bar; }
        error Custom(Bar);
        error Unauthorized();
        error ERC721InsufficientApproval(string operator, uint256 tokenId);
        contract VendingMachine { uint public foo; }"#;
    let abi = load_abi(&build_wasm(src, false)[0].1);

    // Find them in lang_error
    let (error_ty_id, custom_ty_id, erc721_ty_id) = match &abi
        .registry()
        .resolve(abi.spec().lang_error().ty().id)
        .unwrap()
        .type_def
    {
        TypeDef::<PortableForm>::Variant(TypeDefVariant::<PortableForm> { variants }) => {
            let unauthorized = variants.iter().find(|v| v.name == "Unauthorized").unwrap();
            let custom = variants.iter().find(|v| v.name == "Custom").unwrap();
            let erc721 = variants
                .iter()
                .find(|v| v.name == "ERC721InsufficientApproval")
                .unwrap();
            (
                unauthorized.fields[0].ty.id,
                custom.fields[0].ty.id,
                erc721.fields[0].ty.id,
            )
        }
        _ => panic!("unexpected lang_err type def"),
    };

    // Asserts for Unauthorized
    let unauthorized_ty = abi.registry().resolve(error_ty_id).unwrap();
    match &unauthorized_ty.type_def {
        TypeDef::<PortableForm>::Composite(TypeDefComposite::<PortableForm> { fields }) => {
            assert_eq!(unauthorized_ty.path, path!("0x82b42900"));
            assert!(fields.is_empty());
        }
        _ => panic!("expected Unauthorized() type"),
    }

    // Asserts for Custom(Bar)
    let custom_ty = abi.registry().resolve(custom_ty_id).unwrap();
    let custom_ty_id = match &custom_ty.type_def {
        TypeDef::<PortableForm>::Composite(TypeDefComposite::<PortableForm> { fields }) => {
            assert_eq!(custom_ty.path, path!("0x49dfe8ce"));
            fields[0].ty.id
        }
        _ => panic!("expected Foo(Bar) type"),
    };
    let custom_ty = abi.registry().resolve(custom_ty_id).unwrap();
    match &custom_ty.type_def {
        TypeDef::<PortableForm>::Composite(TypeDefComposite { fields }) => {
            assert_eq!(custom_ty.path, path!("Bar"));
            assert_eq!(fields.len(), 2);

            let foo = &fields[0];
            assert_eq!(foo.name.as_ref().unwrap(), "foo");

            let foo_ty = abi.registry().resolve(foo.ty.id).unwrap();
            match &foo_ty.type_def {
                TypeDef::<PortableForm>::Primitive(TypeDefPrimitive::U256) => {
                    assert_eq!(foo_ty.path, path!("uint256"))
                }
                _ => panic!("expected uint256 type"),
            }

            let bar = &fields[1];
            assert_eq!(bar.name.as_ref().unwrap(), "bar");
            let bar_ty = abi.registry().resolve(bar.ty.id).unwrap();
            match &bar_ty.type_def {
                TypeDef::<PortableForm>::Primitive(TypeDefPrimitive::Str) => {
                    assert_eq!(bar_ty.path, path!("string"))
                }
                _ => panic!("expected string type"),
            }
        }
        _ => panic!("expected Foo(Bar) type"),
    };

    // Asserts for ERC721InsufficientApproval
    let erc721_ty = abi.registry().resolve(erc721_ty_id).unwrap();
    let (operator, token_id) = match &erc721_ty.type_def {
        TypeDef::<PortableForm>::Composite(TypeDefComposite::<PortableForm> { fields }) => {
            assert_eq!(erc721_ty.path, path!("0x8799e34a"));
            assert_eq!(fields.len(), 2);
            (&fields[0], &fields[1])
        }
        _ => panic!("expected ERC721InsufficientApproval(string,uint256) type"),
    };

    let operator_ty = abi.registry().resolve(operator.ty.id).unwrap();
    match &operator_ty.type_def {
        TypeDef::<PortableForm>::Primitive(TypeDefPrimitive::Str) => {
            assert_eq!(operator_ty.path, path!("string"))
        }
        _ => panic!("expected string type"),
    }

    let bar_ty = abi.registry().resolve(token_id.ty.id).unwrap();
    match &bar_ty.type_def {
        TypeDef::<PortableForm>::Primitive(TypeDefPrimitive::U256) => {
            assert_eq!(bar_ty.path, path!("uint256"))
        }
        _ => panic!("expected uint256 type"),
    }
}

#[test]
fn overriden_but_not_overloaded_function_not_mangled() {
    let src = r#"abstract contract Base {
        function test() public pure virtual returns (uint8) {
            return 42;
        }
    }
    
    contract TestA is Base {}
    
    contract TestB is Base {
        function test() public pure override returns (uint8) {
            return 42;
        }
    }"#;

    for abi in build_wasm(src, false).iter().map(|(_, abi)| load_abi(abi)) {
        assert_eq!(abi.spec().messages().len(), 1);
        assert_eq!(abi.spec().messages().first().unwrap().label(), "test");
    }
}

#[test]
fn overloaded_but_not_overridden_function_is_mangled() {
    let src = r#"abstract contract Base {
        function test() public pure virtual returns (uint8) {
            return 42;
        }
    
    }
    
    contract TestA is Base {
        function test(uint8 foo) public pure returns (uint8) {
            return foo;
        }
    }"#;

    let mut expected_function_names = HashSet::from(["test_", "test_uint8"]);

    for message_spec in build_wasm(src, false)
        .first()
        .map(|(_, abi)| load_abi(abi))
        .expect("there should be a contract")
        .spec()
        .messages()
    {
        expected_function_names
            .take(message_spec.label().as_str())
            .unwrap_or_else(|| panic!("{} should be present", message_spec.label()));
    }

    assert!(expected_function_names.is_empty());
}
