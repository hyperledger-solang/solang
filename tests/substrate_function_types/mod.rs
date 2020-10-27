extern crate solang;

use super::{build_solidity, first_error, parse_and_resolve};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use solang::Target;

#[test]
fn decls() {
    let ns = parse_and_resolve(
        "contract test {
            function foo() public {
                function() public a;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function type cannot have visibility attribute `public\'"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo() public {
                function() private a;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function type cannot have visibility attribute `private\'"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo() public {
                function() returns (bool) internal a;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "visibility `internal\' cannot be declared after returns"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo() public {
                function() returns (bool) pure a;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "mutability `pure\' cannot be declared after returns"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo() public {
                function() returns (bool x) a;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function type returns cannot be named"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo() public {
                function(address tre) returns (bool) a;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function type parameters cannot be named"
    );

    // internal function types are not allowed in public functions
    let ns = parse_and_resolve(
        "contract test {
            function foo(function(address) pure internal returns (bool) a) public {
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "parameter of type ‘function internal’ not allowed public or external functions"
    );

    let ns = parse_and_resolve(
        "contract test {
            function foo() public returns (function(address) pure internal returns (bool) a) {
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "return type ‘function internal’ not allowed public or external functions"
    );

    let ns = parse_and_resolve(
        "contract test {
            function(address) pure internal returns (bool) public a;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "variable of type internal function cannot be ‘public’"
    );
}

#[test]
fn assign() {
    let ns = parse_and_resolve(
        "contract test {
            function x(int32 arg1) internal {}

            function foo() public {
                function(int32) pure a = x;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function mutability not compatible in conversion from ‘function(int32) internal’ to ‘function(int32) internal pure’"
    );

    let ns = parse_and_resolve(
        "contract test {
            function x(int32 arg1) internal {}

            function foo() public {
                function(int32) view a = x;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function mutability not compatible in conversion from ‘function(int32) internal’ to ‘function(int32) internal view’"
    );

    let ns = parse_and_resolve(
        "contract test {
            function x(int32 arg1) public payable {}

            function foo() public {
                function(int32) a = x;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function mutability not compatible in conversion from ‘function(int32) internal payable’ to ‘function(int32) internal’"
    );

    let ns = parse_and_resolve(
        "contract test {
            function x(int32 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) a = x;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function returns do not match in conversion from ‘function(int32) internal’ to ‘function(int32) internal returns (bool)’"
    );

    let ns = parse_and_resolve(
        "contract test {
            function x(int64 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) returns (bool) a = x;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function arguments do not match in conversion from ‘function(int32) internal returns (bool)’ to ‘function(int64) internal returns (bool)’"
    );
}

#[test]
fn simple_test() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(bool, u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = mul;
                } else {
                    func = add;
                }

                return func(a, b);
            }
        }"##,
    );

    runtime.function("test", Args(true, 100, 10).encode());

    assert_eq!(runtime.vm.output, 1000u32.encode());
}

#[test]
fn internal_function_type_in_contract_storage() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }"##,
    );

    runtime.function("set_op", false.encode());

    runtime.function("test", Args(100, 10).encode());

    assert_eq!(runtime.vm.output, 110u32.encode());
}

#[test]
#[should_panic]
fn internal_function_not_init_called() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }"##,
    );

    // don't call this runtime.function("set_op", false.encode());

    runtime.function("test", Args(100, 10).encode());
}

#[test]
fn base_contract_function() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(bool, u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft is Arith {
            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = Arith.mul;
                } else {
                    func = Arith.add;
                }

                return func(a, b);
            }
        }

        contract Arith {
            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }
        }
        "##,
    );

    runtime.function("test", Args(true, 100, 10).encode());

    assert_eq!(runtime.vm.output, 1000u32.encode());
}

#[test]
fn virtual_contract_function() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Args(bool, u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft is Arith {
            function mul(int32 a, int32 b) internal override returns (int32) {
                return a * b * 10;
            }

            function add(int32 a, int32 b) internal override returns (int32) {
                return a + b + 10;
            }
        }

        contract Arith {
            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = mul;
                } else {
                    func = add;
                }

                return func(a, b);
            }

            function mul(int32 a, int32 b) internal virtual returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal virtual returns (int32) {
                return a + b;
            }
        }
        "##,
    );

    runtime.function("test", Args(true, 100, 10).encode());

    assert_eq!(runtime.vm.output, 10000u32.encode());
}
