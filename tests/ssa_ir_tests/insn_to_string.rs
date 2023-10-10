use crate::num_literal;
use crate::ssa_ir_tests::helpers::{binop_expr, bool_literal, identifier, num_literal, unop_expr};
use solang::codegen::cfg;
use solang::ssa_ir::expr::{BinaryOperator, Expr};
use solang::ssa_ir::insn::Insn;
use solang_parser::pt::Loc;

#[test]
fn test_stringfy_nop_insn() {
    assert_eq!(Insn::Nop.to_string(), "nop;");
}

// ReturnData
#[test]
fn test_stringfy_returndata_insn() {
    assert_eq!(
        Insn::ReturnData {
            data: identifier(0),
            data_len: num_literal!(1),
        }
        .to_string(),
        "return %0 of length uint8(1);"
    );
}

// ReturnCode
#[test]
fn test_stringfy_returncode_insn() {
    assert_eq!(
        Insn::ReturnCode {
            code: cfg::ReturnCode::AbiEncodingInvalid,
        }
        .to_string(),
        "return code \"abi encoding invalid\";"
    );

    assert_eq!(
        Insn::ReturnCode {
            code: cfg::ReturnCode::AccountDataTooSmall,
        }
        .to_string(),
        "return code \"account data too small\";"
    );
}

// Set
#[test]
fn test_stringfy_set_insn() {
    assert_eq!(
        Insn::Set {
            loc: Loc::Codegen,
            res: 122,
            expr: Expr::BinaryExpr {
                loc: Loc::Codegen,
                operator: BinaryOperator::Mul { overflowing: true },
                left: Box::new(num_literal!(1)),
                right: Box::new(identifier(121))
            }
        }
        .to_string(),
        "%122 = uint8(1) (of)* %121;"
    );
}

// Store
#[test]
fn test_stringfy_store_insn() {
    assert_eq!(
        Insn::Store {
            dest: identifier(0),
            data: identifier(1),
        }
        .to_string(),
        "store %1 to %0;"
    );

    // store a number
    assert_eq!(
        Insn::Store {
            dest: identifier(0),
            data: num_literal!(1),
        }
        .to_string(),
        "store uint8(1) to %0;"
    );
}
