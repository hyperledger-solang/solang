// SPDX-License-Identifier: Apache-2.0

use crate::lir_tests::helpers::{
    binop_expr, bool_literal, identifier, new_lir_type, new_printer, new_vartable, num_literal,
    set_tmp, unop_expr,
};
use crate::num_literal;
use crate::stringfy_expr;
use num_bigint::BigInt;
use solang::codegen::Builtin;
use solang::lir::expressions::{BinaryOperator, Expression, UnaryOperator};
use solang::lir::lir_type::{StructType, Type};
use solang::sema::ast::{self, ArrayLength, FormatArg, StringLocation};
use solang_parser::pt::Loc;

#[test]
fn test_stringfy_binary_expr() {
    let mut v = new_vartable();
    for i in 0..100 {
        set_tmp(&mut v, i, Type::Int(16));
    }
    let printer = new_printer(&v);

    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(1),
                BinaryOperator::Add { overflowing: false },
                identifier(2)
            )
        ),
        "int16(%temp.ssa_ir.1) + int16(%temp.ssa_ir.2)"
    );
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(1),
                BinaryOperator::Add { overflowing: true },
                identifier(2)
            )
        ),
        "int16(%temp.ssa_ir.1) (of)+ int16(%temp.ssa_ir.2)"
    );

    // Sub { overflowing: bool },
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(11),
                BinaryOperator::Sub { overflowing: false },
                identifier(12)
            )
        ),
        "int16(%temp.ssa_ir.11) - int16(%temp.ssa_ir.12)"
    );
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(12),
                BinaryOperator::Sub { overflowing: true },
                identifier(13)
            )
        ),
        "int16(%temp.ssa_ir.12) (of)- int16(%temp.ssa_ir.13)"
    );

    // Mul { overflowing: bool },
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(13),
                BinaryOperator::Mul { overflowing: false },
                identifier(14)
            )
        ),
        "int16(%temp.ssa_ir.13) * int16(%temp.ssa_ir.14)"
    );
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(1),
                BinaryOperator::Mul { overflowing: true },
                identifier(9)
            )
        ),
        "int16(%temp.ssa_ir.1) (of)* int16(%temp.ssa_ir.9)"
    );

    // Pow { overflowing: bool },
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                identifier(10),
                BinaryOperator::Pow { overflowing: true },
                identifier(11)
            )
        ),
        "int16(%temp.ssa_ir.10) (of)** int16(%temp.ssa_ir.11)"
    );

    // Div,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::Div, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) / int16(%temp.ssa_ir.2)"
    );

    // UDiv,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(3), BinaryOperator::UDiv, identifier(4))
        ),
        "int16(%temp.ssa_ir.3) (u)/ int16(%temp.ssa_ir.4)"
    );

    // Mod,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(4), BinaryOperator::Mod, identifier(5))
        ),
        "int16(%temp.ssa_ir.4) % int16(%temp.ssa_ir.5)"
    );

    // UMod,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(2), BinaryOperator::UMod, identifier(3))
        ),
        "int16(%temp.ssa_ir.2) (u)% int16(%temp.ssa_ir.3)"
    );

    // Eq,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(2), BinaryOperator::Eq, identifier(4))
        ),
        "int16(%temp.ssa_ir.2) == int16(%temp.ssa_ir.4)"
    );

    // Neq,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(2), BinaryOperator::Neq, identifier(3))
        ),
        "int16(%temp.ssa_ir.2) != int16(%temp.ssa_ir.3)"
    );

    // Lt,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::Lt, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) < int16(%temp.ssa_ir.2)"
    );

    // ULt,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::ULt, identifier(0))
        ),
        "int16(%temp.ssa_ir.1) (u)< int16(%temp.ssa_ir.0)"
    );

    // Lte,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::Lte, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) <= int16(%temp.ssa_ir.2)"
    );

    // ULte,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::ULte, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) (u)<= int16(%temp.ssa_ir.2)"
    );

    // Gt,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::Gt, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) > int16(%temp.ssa_ir.2)"
    );

    // UGt,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::UGt, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) (u)> int16(%temp.ssa_ir.2)"
    );

    // Gte,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::Gte, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) >= int16(%temp.ssa_ir.2)"
    );

    // UGte,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::UGte, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) (u)>= int16(%temp.ssa_ir.2)"
    );

    // BitAnd,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(
                bool_literal(false),
                BinaryOperator::BitAnd,
                bool_literal(true)
            )
        ),
        "false & true"
    );

    // BitOr,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(3), BinaryOperator::BitOr, identifier(4))
        ),
        "int16(%temp.ssa_ir.3) | int16(%temp.ssa_ir.4)"
    );

    // BitXor,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(1), BinaryOperator::BitXor, identifier(2))
        ),
        "int16(%temp.ssa_ir.1) ^ int16(%temp.ssa_ir.2)"
    );

    // Shl,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(3), BinaryOperator::Shl, identifier(4))
        ),
        "int16(%temp.ssa_ir.3) << int16(%temp.ssa_ir.4)"
    );

    // Shr,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(3), BinaryOperator::Shr, identifier(4))
        ),
        "int16(%temp.ssa_ir.3) >> int16(%temp.ssa_ir.4)"
    );

    // UShr,
    assert_eq!(
        stringfy_expr!(
            &printer,
            &binop_expr(identifier(3), BinaryOperator::UShr, identifier(4))
        ),
        "int16(%temp.ssa_ir.3) (u)>> int16(%temp.ssa_ir.4)"
    );
}

#[test]
fn test_stringfy_unary_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Int(16));
    set_tmp(&mut v, 2, Type::Int(16));
    set_tmp(&mut v, 4, Type::Int(16));
    let printer = new_printer(&v);

    // Not,
    assert_eq!(
        stringfy_expr!(
            new_printer(&new_vartable()),
            &unop_expr(UnaryOperator::Not, bool_literal(true))
        ),
        "!true"
    );

    // Neg { overflowing: bool },

    assert_eq!(
        stringfy_expr!(
            &printer,
            &unop_expr(UnaryOperator::Neg { overflowing: false }, identifier(1))
        ),
        "-int16(%temp.ssa_ir.1)"
    );

    assert_eq!(
        stringfy_expr!(
            &printer,
            &unop_expr(UnaryOperator::Neg { overflowing: true }, identifier(2))
        ),
        "(of)-int16(%temp.ssa_ir.2)"
    );

    // BitNot,
    assert_eq!(
        stringfy_expr!(&printer, &unop_expr(UnaryOperator::BitNot, identifier(4))),
        "~int16(%temp.ssa_ir.4)"
    );
}

#[test]
fn test_stringfy_id_expr() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Int(16));

    let printer = new_printer(&v);

    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Id {
                loc: Loc::Codegen,
                id: 1,
            }
        ),
        "int16(%temp.ssa_ir.1)"
    );
}

#[test]
fn test_stringfy_array_literal_expr() {
    // let printer = new_printer();
    let vartab = &new_vartable();
    let printer = new_printer(vartab);
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Array(
                    Box::new(Type::Bool),
                    vec![ast::ArrayLength::Fixed(BigInt::from(2))]
                )),
                dimensions: vec![2],
                values: vec![bool_literal(true), bool_literal(false)],
            }
        ),
        "bool[2] [true, false]"
    );

    // int array
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Array(
                    Box::new(Type::Int(8)),
                    vec![ast::ArrayLength::Fixed(BigInt::from(2))]
                )),
                dimensions: vec![2],
                values: vec![num_literal(1, true, 8), num_literal(2, true, 8)],
            }
        ),
        "int8[2] [int8(1), int8(2)]"
    );

    // uint array
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Array(
                    Box::new(Type::Uint(8)),
                    vec![ast::ArrayLength::Fixed(BigInt::from(2))]
                )),
                dimensions: vec![2],
                values: vec![num_literal!(1), num_literal!(2)],
            }
        ),
        "uint8[2] [uint8(1), uint8(2)]"
    );

    // 2d int array
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Array(
                    Box::new(Type::Int(8)),
                    vec![
                        ast::ArrayLength::Fixed(BigInt::from(2)),
                        ast::ArrayLength::Fixed(BigInt::from(2))
                    ]
                )),
                dimensions: vec![2, 2],
                values: vec![
                    num_literal(1, true, 8),
                    num_literal(2, true, 8),
                    num_literal(3, true, 8),
                    num_literal(4, true, 8)
                ],
            }
        ),
        "int8[2][2] [int8(1), int8(2), int8(3), int8(4)]"
    );

    // 3d int array
    // for example: int8[2][2][2] = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Array(
                    Box::new(Type::Int(8)),
                    vec![
                        ast::ArrayLength::Fixed(BigInt::from(2)),
                        ast::ArrayLength::Fixed(BigInt::from(2)),
                        ast::ArrayLength::Fixed(BigInt::from(2))
                    ]
                )),
                dimensions: vec![2, 2, 2],
                values: vec![
                    num_literal(1, true, 8),
                    num_literal(2, true, 8),
                    num_literal(3, true, 8),
                    num_literal(4, true, 8),
                    num_literal(5, true, 8),
                    num_literal(6, true, 8),
                    num_literal(7, true, 8),
                    num_literal(8, true, 8)
                ],
            }
        ),
        "int8[2][2][2] [int8(1), int8(2), int8(3), int8(4), int8(5), int8(6), int8(7), int8(8)]"
    );

    assert_eq!(
        stringfy_expr!(&printer, &Expression::ConstArrayLiteral {
            loc: Loc::Codegen,
            ty: new_lir_type(Type::Array(
                Box::new(Type::Int(8)),
                vec![
                    ast::ArrayLength::Fixed(BigInt::from(2)),
                    ast::ArrayLength::Fixed(BigInt::from(2)),
                    ast::ArrayLength::Fixed(BigInt::from(2))
                ]
            )),
            dimensions: vec![2, 2, 2],
            values: vec![
                num_literal(1, true, 8),
                num_literal(2, true, 8),
                num_literal(3, true, 8),
                num_literal(4, true, 8),
                num_literal(5, true, 8),
                num_literal(6, true, 8),
                num_literal(7, true, 8),
                num_literal(8, true, 8)
            ],
        }
        ),
        "const int8[2][2][2] [int8(1), int8(2), int8(3), int8(4), int8(5), int8(6), int8(7), int8(8)]"
    );
}

#[test]
fn test_stringfy_bytes_literal_expr() {
    // example: bytes4 hex"41_42_43_44";
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::BytesLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Bytes(4)),
                value: vec![0x41, 0x42, 0x43, 0x44],
            }
        ),
        "bytes4 hex\"41_42_43_44\""
    );
}

#[test]
fn test_stringfy_struct_literal_expr() {
    /*
    example:
    struct S {
        uint x;
        uint y;
    }

    the literal: S(1, 2)

    print: struct { uint8(1), uint8(2) }
    */
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::StructLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Struct(StructType::UserDefined(0))),
                values: vec![num_literal!(1, 8), num_literal!(2, 8)],
            }
        ),
        "struct { uint8(1), uint8(2) }"
    );

    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::StructLiteral {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Struct(StructType::UserDefined(0))),
                values: vec![num_literal!(1, 8), bool_literal(false)],
            }
        ),
        "struct { uint8(1), false }"
    );
}

#[test]
fn test_stringfy_cast_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Uint(8));
    let printer = new_printer(&v);

    // example: uint8(1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Cast {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
                to_ty: new_lir_type(Type::Uint(16)),
            }
        ),
        "(cast uint8(%temp.ssa_ir.1) to uint16)"
    );
}

#[test]
fn test_stringfy_bytes_cast_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Bytes(2));
    let printer = new_printer(&v);

    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::BytesCast {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
                to_ty: new_lir_type(Type::Bytes(4)),
            }
        ),
        "(cast bytes2(%temp.ssa_ir.1) to bytes4)"
    );
}

#[test]
fn test_stringfy_sext_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Int(8));
    let printer = new_printer(&v);

    // example: sign extending a int8 to int16:
    //          %1 of int8 to int16
    //          can be written as: (sext %1 to int16)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::SignExt {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
                to_ty: new_lir_type(Type::Int(16)),
            }
        ),
        "(sext int8(%temp.ssa_ir.1) to int16)"
    );
}

#[test]
fn test_stringfy_zext_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Uint(8));
    let printer = new_printer(&v);

    // example: zero extending a uint8 to uint16:
    //          %1 of uint8 to uint16
    //          can be written as: (zext %1 to int16)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::ZeroExt {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
                to_ty: new_lir_type(Type::Uint(16)),
            }
        ),
        "(zext uint8(%temp.ssa_ir.1) to uint16)"
    );
}

#[test]
fn test_stringfy_trunc_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Uint(16));
    let printer = new_printer(&v);

    // example: truncating a uint16 to uint8:
    //          %1 of uint16 to uint8
    //          can be written as: (trunc uint16 %1 to uint8)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Trunc {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
                to_ty: new_lir_type(Type::Uint(8)),
            }
        ),
        "(trunc uint16(%temp.ssa_ir.1) to uint8)"
    );
}

#[test]
fn test_stringfy_alloc_dyn_bytes() {
    // case1: allocating a dynamic bytes without initializer:
    //        Solidity: bytes memory a = new bytes(10);
    //        rhs print: alloc bytes1[10]
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::AllocDynamicBytes {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Bytes(1)),
                size: Box::new(num_literal!(10)),
                initializer: None,
            }
        ),
        "alloc bytes1[uint8(10)]"
    );

    // case2: allocating a dynamic bytes with initializer:
    //        Solidity: bytes memory a = new bytes(3) { 0x01, 0x02, 0x03 };
    //        rhs print: alloc bytes1[] {0x01, 0x02, 0x03}
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::AllocDynamicBytes {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Bytes(1)),
                size: Box::new(num_literal!(3)),
                initializer: Some(vec![b'\x01', b'\x02', b'\x03']),
            }
        ),
        "alloc bytes1[uint8(3)] {01, 02, 03}"
    );
}

// GetRef
#[test]
fn test_stringfy_get_ref_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Uint(8));
    let printer = new_printer(&v);

    // example: &ptr<uint8>(%temp.ssa_ir.1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::GetRef {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
            }
        ),
        "&uint8(%temp.ssa_ir.1)"
    );
}

// Load
#[test]
fn test_stringfy_load_expr() {
    let mut v = new_vartable();
    set_tmp(&mut v, 1, Type::Ptr(Box::new(Type::Bytes(1))));
    let printer = new_printer(&v);

    // example: *%1
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Load {
                loc: Loc::Codegen,
                operand: Box::new(identifier(1)),
            }
        ),
        "*ptr<bytes1>(%temp.ssa_ir.1)"
    );
}

// StructMember
#[test]
fn test_stringfy_struct_member_expr() {
    let mut v = new_vartable();
    set_tmp(
        &mut v,
        1,
        Type::Ptr(Box::new(Type::Struct(StructType::UserDefined(0)))),
    );
    let printer = new_printer(&v);

    // example: uint8 %1->1
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StructMember {
                loc: Loc::Codegen,
                member: 3,
                operand: Box::new(identifier(1)),
            }
        ),
        "access ptr<struct.0>(%temp.ssa_ir.1) member 3"
    );
}

// Subscript
#[test]
fn test_stringfy_subscript_expr() {
    let mut v = new_vartable();

    set_tmp(
        &mut v,
        1,
        Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(8)),
            vec![ArrayLength::Fixed(BigInt::from(2))],
        ))),
    );
    set_tmp(
        &mut v,
        2,
        Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(8)),
            vec![ArrayLength::Dynamic],
        ))),
    );
    set_tmp(
        &mut v,
        3,
        Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(8)),
            vec![ArrayLength::AnyFixed],
        ))),
    );
    let printer = new_printer(&v);

    // example: ptr<uint8[2]> %1[uint8(0)]
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Subscript {
                loc: Loc::Codegen,
                arr: Box::new(identifier(1)),
                index: Box::new(num_literal!(0)),
            }
        ),
        "ptr<uint8[2]>(%temp.ssa_ir.1)[uint8(0)]"
    );

    // example: ptr<uint8[]> %1[uint8(0)]
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Subscript {
                loc: Loc::Codegen,
                arr: Box::new(identifier(2)),
                index: Box::new(num_literal!(0)),
            }
        ),
        "ptr<uint8[]>(%temp.ssa_ir.2)[uint8(0)]"
    );

    // example: ptr<uint8[?]> %1[uint8(0)]
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Subscript {
                loc: Loc::Codegen,
                arr: Box::new(identifier(3)),
                index: Box::new(num_literal!(0)),
            }
        ),
        "ptr<uint8[?]>(%temp.ssa_ir.3)[uint8(0)]"
    );
}

// AdvancePointer
#[test]
fn test_stringfy_advance_pointer_expr() {
    let mut v = new_vartable();
    set_tmp(
        &mut v,
        1,
        Type::Ptr(Box::new(Type::Struct(StructType::UserDefined(0)))),
    );
    set_tmp(&mut v, 2, Type::Uint(8));
    let printer = new_printer(&v);

    // example: ptr_add(%1, %2)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::AdvancePointer {
                loc: /*missing from cfg*/ Loc::Codegen,
                pointer: Box::new(identifier(1)),
                bytes_offset: Box::new(identifier(2)),
            }
        ),
        "ptr_add(ptr<struct.0>(%temp.ssa_ir.1), uint8(%temp.ssa_ir.2))"
    );

    // example: ptr_add(%1, uint8(1))
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::AdvancePointer {
                loc: /*missing from cfg*/ Loc::Codegen,
                pointer: Box::new(identifier(1)),
                bytes_offset: Box::new(num_literal!(1)),
            }
        ),
        "ptr_add(ptr<struct.0>(%temp.ssa_ir.1), uint8(1))"
    );
}

// FunctionArg
#[test]
fn test_stringfy_function_arg_expr() {
    // example: the 2nd arg of type uint8
    //          (uint8 arg#2)
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::FunctionArg {
                loc: Loc::Codegen,
                ty: new_lir_type(Type::Uint(8)),
                arg_no: 2,
            }
        ),
        "uint8(arg#2)"
    );
}

// FormatString
#[test]
fn test_stringfy_format_string_expr() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bytes(4));
    set_tmp(&mut v, 2, Type::Int(16));
    set_tmp(&mut v, 3, Type::Uint(8));
    set_tmp(&mut v, 4, Type::Uint(32));

    let printer = new_printer(&v);
    // case1: spec is empty:
    //        fmt_str(%1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::FormatString {
                loc: Loc::Codegen,
                args: vec![(FormatArg::StringLiteral, identifier(1))]
            }
        ),
        "fmt_str(bytes4(%temp.ssa_ir.1))"
    );
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::FormatString {
                loc: Loc::Codegen,
                args: vec![(FormatArg::Default, identifier(2))]
            }
        ),
        "fmt_str(int16(%temp.ssa_ir.2))"
    );

    // case2: spec is binary:
    //        fmt_str(:b %1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::FormatString {
                loc: Loc::Codegen,
                args: vec![(FormatArg::Binary, identifier(2))]
            }
        ),
        "fmt_str(:b int16(%temp.ssa_ir.2))"
    );

    // case3: spec is hex:
    //        fmt_str(:x int16(%temp.ssa_ir.1))
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::FormatString {
                loc: Loc::Codegen,
                args: vec![(FormatArg::Hex, identifier(2))]
            }
        ),
        "fmt_str(:x int16(%temp.ssa_ir.2))"
    );

    // mixed case:
    // fmt_str(%1, %2, :b %2, :x %3)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::FormatString {
                loc: Loc::Codegen,
                args: vec![
                    (FormatArg::StringLiteral, identifier(1)),
                    (FormatArg::Default, identifier(2)),
                    (FormatArg::Binary, identifier(3)),
                    (FormatArg::Hex, identifier(4))
                ]
            }
        ),
        "fmt_str(bytes4(%temp.ssa_ir.1), int16(%temp.ssa_ir.2), :b uint8(%temp.ssa_ir.3), :x uint32(%temp.ssa_ir.4))"
    );
}

// InternalFunctionCfg
#[test]
fn test_stringfy_internal_function_cfg_expr() {
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::InternalFunctionCfg { loc: /*missing from cfg*/ Loc::Codegen, cfg_no: 123 }
        ),
        "function#123"
    );
}

// Keccak256
#[test]
fn test_stringfy_keccak256_expr() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bytes(4));
    set_tmp(&mut v, 2, Type::Bytes(4));

    let printer = new_printer(&v);
    // example: keccak256(%1, %2)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Keccak256 {
                loc: Loc::Codegen,
                args: vec![identifier(1), identifier(2)],
            }
        ),
        "keccak256(bytes4(%temp.ssa_ir.1), bytes4(%temp.ssa_ir.2))"
    );
}

// StringCompare
#[test]
fn test_stringfy_string_compare_expr() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bytes(4));
    set_tmp(&mut v, 2, Type::Bytes(4));
    set_tmp(&mut v, 3, Type::Bytes(3));

    let printer = new_printer(&v);
    // case1: strcmp(%1, %2)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StringCompare {
                loc: Loc::Codegen,
                left: StringLocation::RunTime(Box::new(identifier(1))),
                right: StringLocation::RunTime(Box::new(identifier(2))),
            }
        ),
        "strcmp(bytes4(%temp.ssa_ir.1), bytes4(%temp.ssa_ir.2))"
    );

    // case2: strcmp("[97, 98, 99]", %1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StringCompare {
                loc: Loc::Codegen,
                left: StringLocation::CompileTime(vec![b'a', b'b', b'c']),
                right: StringLocation::RunTime(Box::new(identifier(3))),
            }
        ),
        "strcmp(\"[97, 98, 99]\", bytes3(%temp.ssa_ir.3))"
    );

    // case3: strcmp(%1, "[97, 98, 99]")
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StringCompare {
                loc: Loc::Codegen,
                left: StringLocation::RunTime(Box::new(identifier(3))),
                right: StringLocation::CompileTime(vec![b'a', b'b', b'c']),
            }
        ),
        "strcmp(bytes3(%temp.ssa_ir.3), \"[97, 98, 99]\")"
    );
}

// StringConcat
#[test]
fn test_stringfy_string_concat() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bytes(4));
    set_tmp(&mut v, 2, Type::Bytes(2));

    let printer = new_printer(&v);
    // case1: strcat(%1, %2)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StringConcat {
                loc: Loc::Codegen,
                left: StringLocation::RunTime(Box::new(identifier(1))),
                right: StringLocation::RunTime(Box::new(identifier(2))),
            }
        ),
        "strcat(bytes4(%temp.ssa_ir.1), bytes2(%temp.ssa_ir.2))"
    );
    // case2: strcat("[97, 98, 99]", %1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StringConcat {
                loc: Loc::Codegen,
                left: StringLocation::CompileTime(vec![b'a', b'b', b'c']),
                right: StringLocation::RunTime(Box::new(identifier(1))),
            }
        ),
        "strcat(\"[97, 98, 99]\", bytes4(%temp.ssa_ir.1))"
    );
    // case3: strcat(%1, "[97, 98, 99]")
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StringConcat {
                loc: Loc::Codegen,
                left: StringLocation::RunTime(Box::new(identifier(1))),
                right: StringLocation::CompileTime(vec![b'a', b'b', b'c']),
            }
        ),
        "strcat(bytes4(%temp.ssa_ir.1), \"[97, 98, 99]\")"
    );
}

// StorageArrayLength
#[test]
fn test_stringfy_storage_array_length() {
    let mut v = new_vartable();

    set_tmp(
        &mut v,
        1,
        Type::StoragePtr(
            false,
            Box::new(Type::Array(
                Box::new(Type::Uint(8)),
                vec![ArrayLength::Dynamic],
            )),
        ),
    );

    let printer = new_printer(&v);

    // example: storage_arr_len(uint8[] %1)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::StorageArrayLength {
                loc: Loc::Codegen,
                array: Box::new(identifier(1)),
            }
        ),
        "storage_arr_len(storage_ptr<uint8[]>(%temp.ssa_ir.1))"
    );
}

// ReturnData
#[test]
fn test_stringfy_return_data() {
    // example: ret_data
    assert_eq!(
        stringfy_expr!(
            &new_printer(&new_vartable()),
            &Expression::ReturnData { loc: Loc::Codegen }
        ),
        "(extern_call_ret_data)"
    );
}

#[test]
fn test_stringfy_builtin() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Int(16));
    set_tmp(&mut v, 2, Type::Int(16));

    let printer = new_printer(&v);
    // example: builtin "addmod"(%1, %2, 0x100)
    assert_eq!(
        stringfy_expr!(
            &printer,
            &Expression::Builtin {
                loc: Loc::Codegen,
                kind: Builtin::AddMod,
                args: vec![identifier(1), identifier(2), num_literal!(0x100, 16)],
            }
        ),
        "builtin: AddMod(int16(%temp.ssa_ir.1), int16(%temp.ssa_ir.2), uint16(256))"
    );
}
