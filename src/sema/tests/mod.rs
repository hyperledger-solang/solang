// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]
use crate::sema::ast::{Expression, Parameter, Statement, TryCatch, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::unescape;
use crate::sema::yul::ast::InlineAssembly;
use crate::{parse_and_resolve, sema::ast, FileResolver, Target};
use solang_parser::pt::Loc;
use std::ffi::OsStr;

pub(crate) fn parse(src: &'static str) -> ast::Namespace {
    let mut cache = FileResolver::new();
    cache.set_file_contents("test.sol", src.to_string());

    let ns = parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM);
    ns.print_diagnostics_in_plain(&cache, false);
    ns
}

#[test]
fn test_unescape() {
    let s = r#"\u00f3"#;
    let mut vec = Diagnostics::default();
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![0xc3, 0xb3]);
    let s = r#"\xff"#;
    let res = unescape(s, 0, 0, &mut vec);
    assert!(vec.is_empty());
    assert_eq!(res, vec![255]);
}

#[test]
fn test_statement_reachable() {
    let loc = Loc::File(0, 1, 2);
    let test_cases: Vec<(Statement, bool)> = vec![
        (Statement::Underscore(loc), true),
        (
            Statement::Destructure(loc, vec![], Expression::BoolLiteral(loc, true)),
            true,
        ),
        (
            Statement::VariableDecl(
                loc,
                0,
                Parameter {
                    loc,
                    id: None,
                    ty: Type::Bool,
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                },
                None,
            ),
            true,
        ),
        (
            Statement::Emit {
                loc,
                event_no: 0,
                event_loc: Loc::Builtin,
                args: vec![],
            },
            true,
        ),
        (
            Statement::Delete(loc, Type::Bool, Expression::BoolLiteral(loc, true)),
            true,
        ),
        (Statement::Continue(loc), false),
        (Statement::Break(loc), false),
        (Statement::Return(loc, None), false),
        (
            Statement::If(
                loc,
                false,
                Expression::BoolLiteral(loc, false),
                vec![],
                vec![],
            ),
            false,
        ),
        (
            Statement::While(loc, true, Expression::BoolLiteral(loc, false), vec![]),
            true,
        ),
        (
            Statement::DoWhile(loc, false, vec![], Expression::BoolLiteral(loc, true)),
            false,
        ),
        (
            Statement::Expression(loc, true, Expression::BoolLiteral(loc, false)),
            true,
        ),
        (
            Statement::For {
                loc,
                reachable: false,
                init: vec![],
                cond: None,
                next: vec![],
                body: vec![],
            },
            false,
        ),
        (
            Statement::TryCatch(
                loc,
                true,
                TryCatch {
                    expr: Expression::BoolLiteral(loc, false),
                    returns: vec![],
                    ok_stmt: vec![],
                    errors: vec![],
                    catch_param: None,
                    catch_param_pos: None,
                    catch_stmt: vec![],
                },
            ),
            true,
        ),
        (
            Statement::Assembly(
                InlineAssembly {
                    loc,
                    body: vec![],
                    functions: std::ops::Range { start: 0, end: 0 },
                },
                false,
            ),
            false,
        ),
    ];

    for (test_case, expected) in test_cases {
        assert_eq!(test_case.reachable(), expected);
    }
}

#[test]
fn constant_overflow_checks() {
    let file = r#"
    contract test_contract {
        function test_params(uint8 usesa, int8 sesa) public returns(uint8) {
            return usesa;
        }
    
        function test_add(int8 input) public returns (uint8) {
            // value 133 does not fit into type int8.
            int8 add_ovf = 127 + 6;
    
            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            uint8 negative = 3 - 4;
    
            // value 133 does not fit into type int8.
            int8 mixed = 126 + 7 + input;
    
            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            return 1 - 2;
        }
    
        function test_mul(int8 input) public {
            // value 726 does not fit into type int8.
            int8 mul_ovf = 127 * 6;
    
            // value 882 does not fit into type int8.
            int8 mixed = 126 * 7 * input;
        }
    
        function test_shift(int8 input) public {
            // value 128 does not fit into type int8.
            int8 mul_ovf = 1 << 7;
    
            // value 128 does not fit into type int8.
            int8 mixed = (1 << 7) + input;
        }
    
        function test_call() public {
            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            // value 129 does not fit into type int8.
            test_params(1 - 2, 127 + 2);
    
            // negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.
            // value 129 does not fit into type int8.
            test_params({usesa: 1 - 2, sesa: 127 + 2});
        }

        function test_builtin (bytes input) public{

            // value 4294967296 does not fit into type uint32.
            int16 sesa = input.readInt16LE(4294967296);
        }

        function test_for_loop () public {
            for (int8 i = 125 + 5; i < 300 ; i++) {
            }
        }

        function composite(int8 a, bytes input) public{

            uint8 sesa = 500- 400 + test_params(100+200, 0) + (200+101) + input.readUint8(4294967296);
            int8 seas = (120 + 120) + a + (120 + 125);  
            uint8 b = 255 - 255/5 ;
        }
    }
    
        "#;
    let ns = parse(file);
    let errors = ns.diagnostics.errors();

    assert_eq!(errors[0].message, "value 133 does not fit into type int8.");
    assert_eq!(errors[1].message, "negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(errors[2].message, "value 133 does not fit into type int8.");
    assert_eq!(errors[4].message, "value 762 does not fit into type int8.");
    assert_eq!(errors[5].message, "value 882 does not fit into type int8.");
    assert_eq!(errors[6].message, "value 128 does not fit into type int8.");
    assert_eq!(errors[7].message, "value 128 does not fit into type int8.");
    assert_eq!(errors[8].message, "negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(errors[9].message, "value 129 does not fit into type int8.");
    assert_eq!(errors[10].message, "negative value -1 does not fit into type uint8. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(errors[11].message, "value 129 does not fit into type int8.");
    assert_eq!(
        errors[12].message,
        "value 4294967296 does not fit into type uint32."
    );
    assert_eq!(errors[13].message, "value 130 does not fit into type int8.");
    assert_eq!(
        errors[14].message,
        "value 300 does not fit into type uint8."
    );
    assert_eq!(
        errors[15].message,
        "value 301 does not fit into type uint8."
    );
    assert_eq!(
        errors[16].message,
        "value 4294967296 does not fit into type uint32."
    );
    assert_eq!(errors[17].message, "value 240 does not fit into type int8.");
    assert_eq!(errors[18].message, "value 245 does not fit into type int8.");
}

#[test]
fn test_types() {
    let file = r#"
    contract test_contract {
    
        function test_types32(bytes input ) public {
            // value 2147483648 does not fit into type int32.
            int32 add_ovf = 2147483647 + 1;

            // value 2147483648 does not fit into type int32.
            int32 add_normal = 2147483647 + 0;

            // value 2147483648 does not fit into type int32.
            int32 mixed = 2147483647 + 1 + input.readInt32LE(2);
        }
    

        function test_types64(bytes input ) public {
            // value 9223372036854775808 does not fit into type int64.
            int64 add_ovf = 9223372036854775807 + 1 ;

            int64 add_normal = 9223372036854775807;

            // value 9223372036854775808 does not fit into type int64.
            int64 mixed = 9223372036854775807 + 1 + input.readInt64LE(2);

            // value 18446744073709551616 does not fit into type uint64.
            uint64 pow_ovf = 2 ** 64  ;

            uint64 normal_pow = (2**64) -1 ;
        }



        function test_types_128_256(bytes input ) public {
            
            while (true) {
                // value 340282366920938463463374607431768211456 does not fit into type uint64.
                uint128 ovf = 2 ** 128 ;
                uint128 normal = 2** 128 -1 ;
            }
            uint128[] arr;
            // negative value -1 does not fit into type uint32. Cannot implicitly convert signed literal to unsigned type.
            // value 340282366920938463463374607431768211456 does not fit into type uint128.
            uint128 access = arr[1-2] + 1 + (2**128);
            // value 3000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 does not fit into type uint256.
            uint256 num = 3e255;

            uint256 num_2 = 115792089237316195423570985008687907853269984665640564039457584007913129639935 * 4;
        }
        
    }
        "#;
    let ns = parse(file);
    let errors = ns.diagnostics.errors();

    assert_eq!(
        errors[0].message,
        "value 2147483648 does not fit into type int32."
    );
    assert_eq!(
        errors[1].message,
        "value 2147483648 does not fit into type int32."
    );
    assert_eq!(
        errors[2].message,
        "value 9223372036854775808 does not fit into type int64."
    );
    assert_eq!(
        errors[3].message,
        "value 9223372036854775808 does not fit into type int64."
    );
    assert_eq!(
        errors[4].message,
        "value 18446744073709551616 does not fit into type uint64."
    );
    assert_eq!(
        errors[5].message,
        "value 340282366920938463463374607431768211456 does not fit into type uint128."
    );
    assert_eq!(errors[6].message, "negative value -1 does not fit into type uint32. Cannot implicitly convert signed literal to unsigned type.");
    assert_eq!(
        errors[7].message,
        "value 340282366920938463463374607431768211456 does not fit into type uint128."
    );

    assert_eq!(errors[8].message, "value 3000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 does not fit into type uint256.");

    assert_eq!(errors[9].message, "value 463168356949264781694283940034751631413079938662562256157830336031652518559740 does not fit into type uint256.");
}
