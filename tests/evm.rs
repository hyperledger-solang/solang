// SPDX-License-Identifier: Apache-2.0

use rayon::prelude::*;
use solang::{file_resolver::FileResolver, parse_and_resolve, sema::ast, Target};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

mod evm_tests;

fn test_solidity(src: &str) -> ast::Namespace {
    let mut cache = FileResolver::default();

    cache.set_file_contents("test.sol", src.to_string());

    let ns = parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM);

    ns.print_diagnostics_in_plain(&cache, false);

    ns
}

#[test]
fn address() {
    let ns = test_solidity(
        "
        contract address_tester {
            function encode_const() public returns (address) {
                return 0x52908400098527886E0F7030069857D2E4169EE7;
            }

            function test_arg(address foo) public {
                assert(foo == 0x27b1fdb04752bbc536007a920d24acb045561c26);

                // this literal is a number
                int x = 0x27b1fdb047_52bbc536007a920d24acb045561C26;
                assert(int(foo) == x);
            }

            function allones() public returns (address) {
                return address(1);
            }
        }",
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn try_catch() {
    let ns = test_solidity(
        r#"
        contract b {
            int32 x;

            constructor(int32 a) public {
                x = a;
            }

            function get_x(int32 t) public returns (int32) {
                if (t == 0) {
                    revert("cannot be zero");
                }
                return x * t;
            }
        }

        contract c {
            b x;

            constructor() public {
                x = new b(102);
            }

            function test() public returns (int32) {
                int32 state = 0;
                try x.get_x(0) returns (int32 l) {
                    state = 1;
                } catch Error(string err) {
                    if (err == "cannot be zero") {
                        state = 2;
                    } else {
                        state = 3;
                    }
                } catch (bytes ) {
                    state = 4;
                }

                return state;
            }
        }"#,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn selfdestruct() {
    let ns = test_solidity(
        r##"
        contract other {
            function goaway(address payable recipient) public returns (bool) {
                selfdestruct(recipient);
            }
        }

        contract c {
            other o;
            function step1() public {
                o = new other{value: 511}();
            }

            function step2() public {
                bool foo = o.goaway(payable(address(this)));
            }
        }"##,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn eth_builtins() {
    let ns = test_solidity(
        r#"
contract testing  {
    function test_address() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := a
        }
    }

    function test_balance() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := balance(a)
        }
    }

    function test_selfbalance() public view returns (uint256 ret) {
        assembly {
            let a := selfbalance()
            ret := a
        }
    }

    function test_caller() public view returns (uint256 ret) {
        assembly {
            let a := caller()
            ret := a
        }
    }

    function test_callvalue() public view returns (uint256 ret) {
        assembly {
            let a := callvalue()
            ret := a
        }
    }

    function test_extcodesize() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := extcodesize(a)
        }
    }
}
"#,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn ethereum_solidity_tests() {
    let error_matcher =
        regex::Regex::new(r"// ----\r?\n(//\s+Warning \d+: .*\n)*//\s+\w+Error( \d+)?: (.*)")
            .unwrap();

    let entries = WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/solidity/test/libsolidity/semanticTests"),
    )
    .into_iter()
    .chain(WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/solidity/test/libsolidity/syntaxTests"),
    ));

    let errors: usize = entries
        .par_bridge()
        .filter_map(|e| {
            let entry = e.unwrap();
            let file_name = entry.file_name().to_string_lossy();

            // FIXME: max_depth_reached_4.sol causes a stack overflow in resolve_expression.rs
            // FIXME: others listed explicitly cause panics and need fixing
            if !file_name.ends_with("max_depth_reached_4.sol")
                && !file_name.ends_with("invalid_utf8_sequence.sol")
                // Bug in solc: https://github.com/ethereum/solidity/issues/11573
                && !file_name
                    .ends_with("internal_library_function_attached_to_string_accepting_storage.sol")
                && file_name.ends_with(".sol")
            {
                Some(entry)
            } else {
                None
            }
        })
        .map(|entry| {
            let path = entry.path().parent().unwrap();

            let source = fs::read_to_string(entry.path()).unwrap();

            let expect_error = error_matcher
                .captures(&source)
                .map(|captures| captures.get(3).unwrap().as_str());

            let (mut cache, names) = set_file_contents(&source, entry.path());

            cache.add_import_path(path);

            let errors: usize = names
                .iter()
                .map(|name| {
                    let ns = parse_and_resolve(OsStr::new(&name), &mut cache, Target::EVM);

                    if ns.diagnostics.any_errors() {
                        if expect_error.is_none() {
                            println!("file: {} name:{}", entry.path().display(), name);

                            ns.print_diagnostics_in_plain(&cache, false);

                            1
                        } else {
                            0
                        }
                    } else if let Some(error) = expect_error {
                        println!("file: {} name:{}", entry.path().display(), name);

                        println!("expecting error {error}");

                        1
                    } else {
                        0
                    }
                })
                .sum();

            errors
        })
        .sum();

    assert_eq!(errors, 897);
}

fn set_file_contents(source: &str, path: &Path) -> (FileResolver, Vec<String>) {
    let mut cache = FileResolver::default();
    let mut name = path.to_string_lossy().to_string();
    let mut names = Vec::new();
    let mut contents = String::new();
    let source_delimiter = regex::Regex::new(r"==== Source: (.*) ====").unwrap();
    let external_source_delimiter = regex::Regex::new(r"==== ExternalSource: (.*) ====").unwrap();
    let equals = regex::Regex::new("([a-zA-Z0-9_]+)=(.*)").unwrap();

    for line in source.lines() {
        if let Some(cap) = source_delimiter.captures(line) {
            if !contents.is_empty() {
                cache.set_file_contents(&name, contents);
                names.push(name);
                name = String::new();
            }
            cap.get(1).unwrap().as_str().clone_into(&mut name);
            if name == "////" {
                "test.sol".clone_into(&mut name);
            }
            contents = String::new();
        } else if let Some(cap) = external_source_delimiter.captures(line) {
            let filename = cap.get(1).unwrap().as_str();
            let mut name = filename.to_owned();
            if let Some(cap) = equals.captures(filename) {
                let mut ext = path.parent().unwrap().to_path_buf();
                ext.push(cap.get(2).unwrap().as_str());
                cap.get(1).unwrap().as_str().clone_into(&mut name);
                let source = fs::read_to_string(ext).unwrap();
                cache.set_file_contents(&name, source);
            }
            // else rely on file resolver to import stuff
        } else {
            contents.push_str(line);
            contents.push('\n');
        }
    }

    if !contents.is_empty() {
        cache.set_file_contents(&name, contents);
        names.push(name);
    }

    cache.add_import_path(&PathBuf::from(""));

    (cache, names)
}
