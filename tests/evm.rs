// SPDX-License-Identifier: Apache-2.0

use solang::{file_resolver::FileResolver, parse_and_resolve, sema::ast, Target};
use std::{ffi::OsStr, fs, path::Path};
use walkdir::WalkDir;

fn test_solidity(src: &str) -> ast::Namespace {
    let mut cache = FileResolver::new();

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
        r##"
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
        }"##,
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
    let error_matcher = regex::Regex::new(r"// ----\r?\n// \w+Error( \d+)?:").unwrap();

    let semantic_tests = WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/solidity/test/libsolidity/semanticTests"),
    )
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    let syntax_tests = WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/solidity/test/libsolidity/syntaxTests"),
    )
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    let errors: usize = semantic_tests
        .into_iter()
        .chain(syntax_tests.into_iter())
        .map(|entry| {
            let file_name = entry.file_name().to_string_lossy();
            let path = entry.path().parent().unwrap();

            // FIXME: max_depth_reached_4.sol causes a stack overflow in resolve_expression.rs
            // FIXNE: others listed explicitly cause panics and need fixing
            if !file_name.ends_with("max_depth_reached_4.sol")
                && !file_name.ends_with("invalid_utf8_sequence.sol")
                && !file_name.ends_with("basefee_berlin_function.sol")
                && !file_name.ends_with("inline_assembly_embedded_function_call.sol")
                && !file_name.ends_with("cannot_be_function_call.sol")
                && !file_name.ends_with("complex_cyclic_constant.sol")
                && !file_name.ends_with("cyclic_constant.sol")
                && !file_name.ends_with("pure_functions.sol")
                && !file_name.ends_with("pure_non_rational.sol")
                && !file_name.ends_with("linkersymbol_function.sol")
                && !file_name.ends_with("370_shift_constant_left_excessive_rvalue.sol")
                && file_name.ends_with(".sol")
            {
                let source = fs::read_to_string(entry.path()).unwrap();

                let expect_error = error_matcher.is_match(&source);

                let (mut cache, names) = set_file_contents(&source, path);

                cache.add_import_path(path).unwrap();

                let errors: usize = names
                    .iter()
                    .map(|name| {
                        let ns = parse_and_resolve(OsStr::new(&name), &mut cache, Target::EVM);

                        if ns.diagnostics.any_errors() {
                            if !expect_error {
                                println!("file: {}", entry.path().display());

                                ns.print_diagnostics_in_plain(&cache, false);

                                1
                            } else {
                                0
                            }
                        } else if expect_error {
                            println!("file: {}", entry.path().display());

                            println!("expecting error, none found");

                            1
                        } else {
                            0
                        }
                    })
                    .sum();

                errors
            } else {
                0
            }
        })
        .sum();

    assert_eq!(errors, 1062);
}

fn set_file_contents(source: &str, path: &Path) -> (FileResolver, Vec<String>) {
    let mut cache = FileResolver::new();
    let mut name = "test.sol".to_owned();
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
            }
            name = cap.get(1).unwrap().as_str().to_owned();
            if name == "////" {
                name = "test.sol".to_owned();
            }
            contents = String::new();
        } else if let Some(cap) = external_source_delimiter.captures(line) {
            let mut name = cap.get(1).unwrap().as_str().to_owned();
            if let Some(cap) = equals.captures(&name) {
                let mut ext = path.to_path_buf();
                ext.push(cap.get(2).unwrap().as_str());
                name = cap.get(1).unwrap().as_str().to_owned();
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

    (cache, names)
}
