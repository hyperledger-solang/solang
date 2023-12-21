// SPDX-License-Identifier: Apache-2.0

use crate::lir_tests::helpers::{identifier, new_printer, new_vartable, num_literal, set_tmp};
use crate::{num_literal, stringfy_insn};
use num_bigint::BigInt;
use solang::codegen::cfg;
use solang::lir::expressions::{BinaryOperator, Expression};
use solang::lir::instructions::Instruction;
use solang::lir::lir_type::{InternalCallTy, PhiInput, StructType, Type};
use solang::sema::ast::{ArrayLength, CallTy};
use solang_parser::pt::Loc;

#[test]
fn test_stringfy_nop_insn() {
    assert_eq!(
        stringfy_insn!(&new_printer(&new_vartable()), &Instruction::Nop),
        "nop;"
    );
}

// ReturnData
#[test]
fn test_stringfy_returndata_insn() {
    let mut v = new_vartable();
    set_tmp(&mut v, 0, Type::Bytes(1));
    let printer = new_printer(&v);

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ReturnData {
                loc: /*missing from cfg*/ Loc::Codegen,
                data: identifier(0),
                data_len: num_literal!(1),
            }
        ),
        "return_data bytes1(%temp.ssa_ir.0) of length uint8(1);"
    );
}

// ReturnCode
#[test]
fn test_stringfy_returncode_insn() {
    assert_eq!(
        stringfy_insn!(
            &new_printer(&new_vartable()),
            &Instruction::ReturnCode {
                loc: /*missing from cfg*/ Loc::Codegen,
                code: cfg::ReturnCode::AbiEncodingInvalid,
            }
        ),
        "return_code \"abi encoding invalid\";"
    );

    assert_eq!(
        stringfy_insn!(
            &new_printer(&new_vartable()),
            &Instruction::ReturnCode {
                loc: /*missing from cfg*/ Loc::Codegen,
                code: cfg::ReturnCode::AccountDataTooSmall,
            }
        ),
        "return_code \"account data too small\";"
    );
}

// Set
#[test]
fn test_stringfy_set_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 121, Type::Uint(8));
    set_tmp(&mut v, 122, Type::Uint(8));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Set {
                loc: Loc::Codegen,
                res: 122,
                expr: Expression::BinaryExpr {
                    loc: Loc::Codegen,
                    operator: BinaryOperator::Mul { overflowing: true },
                    left: Box::new(num_literal!(1)),
                    right: Box::new(identifier(121))
                }
            }
        ),
        "uint8 %temp.ssa_ir.122 = uint8(1) (of)* uint8(%temp.ssa_ir.121);"
    );
}

// Store
#[test]
fn test_stringfy_store_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 0, Type::Ptr(Box::new(Type::Uint(8))));
    set_tmp(&mut v, 1, Type::Uint(8));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Store {
                loc: /*missing from cfg*/ Loc::Codegen,
                dest: identifier(0),
                data: identifier(1),
            }
        ),
        "store uint8(%temp.ssa_ir.1) to ptr<uint8>(%temp.ssa_ir.0);"
    );

    // store a number
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Store {
                loc: /*missing from cfg*/ Loc::Codegen,
                dest: identifier(0),
                data: num_literal!(1),
            }
        ),
        "store uint8(1) to ptr<uint8>(%temp.ssa_ir.0);"
    );
}

// PushMemory
#[test]
fn test_stringfy_push_memory_insn() {
    let mut v = new_vartable();

    set_tmp(
        &mut v,
        3,
        Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(32)),
            vec![ArrayLength::Fixed(BigInt::from(3))],
        ))),
    );
    set_tmp(&mut v, 101, Type::Uint(32));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PushMemory {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: 101,
                array: 3,
                value: num_literal!(1, 32),
            }
        ),
        "uint32 %temp.ssa_ir.101 = push_mem ptr<uint32[3]>(%temp.ssa_ir.3) uint32(1);"
    );
}

#[test]
fn test_stringfy_pop_memory_insn() {
    let mut v = new_vartable();

    set_tmp(
        &mut v,
        3,
        Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(32)),
            vec![ArrayLength::Fixed(BigInt::from(3))],
        ))),
    );
    set_tmp(&mut v, 101, Type::Uint(32));
    let printer = new_printer(&v);

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PopMemory {
                res: 101,
                array: 3,
                loc: Loc::Codegen,
            }
        ),
        "uint32 %temp.ssa_ir.101 = pop_mem ptr<uint32[3]>(%temp.ssa_ir.3);"
    );
}

// LoadStorage
#[test]
fn test_stringfy_load_storage_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 101, Type::Uint(32));
    set_tmp(&mut v, 3, Type::StoragePtr(false, Box::new(Type::Uint(32))));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::LoadStorage {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: 101,
                storage: identifier(3)
            }
        ),
        "uint32 %temp.ssa_ir.101 = load_storage storage_ptr<uint32>(%temp.ssa_ir.3);"
    );
}

#[test]
fn test_stringfy_clear_storage_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 3, Type::StoragePtr(false, Box::new(Type::Uint(32))));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ClearStorage {
                loc: /*missing from cfg*/ Loc::Codegen,
                storage: identifier(3)
            }
        ),
        "clear_storage storage_ptr<uint32>(%temp.ssa_ir.3);"
    );
}

#[test]
fn test_stringfy_set_storage_insn() {
    let mut v = new_vartable();

    set_tmp(
        &mut v,
        1,
        Type::StoragePtr(false, Box::new(Type::Uint(256))),
    );
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::SetStorage {
                loc: /*missing from cfg*/ Loc::Codegen,
                value: num_literal(13445566, false, 256),
                storage: identifier(1)
            }
        ),
        "set_storage storage_ptr<uint256>(%temp.ssa_ir.1) uint256(13445566);"
    );
}

#[test]
fn test_stringfy_set_storage_bytes_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bytes(32));
    set_tmp(
        &mut v,
        2,
        Type::StoragePtr(false, Box::new(Type::Bytes(32))),
    );
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::SetStorageBytes {
                loc: /*missing from cfg*/ Loc::Codegen,
                value: identifier(1),
                storage: identifier(2),
                offset: num_literal!(3)
            }
        ),
        "set_storage_bytes storage_ptr<bytes32>(%temp.ssa_ir.2) offset:uint8(3) value:bytes32(%temp.ssa_ir.1);"
    );
}

#[test]
fn test_stringfy_push_storage_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 101, Type::Uint(32));
    set_tmp(
        &mut v,
        3,
        Type::StoragePtr(
            false,
            Box::new(Type::Array(
                Box::new(Type::Uint(32)),
                vec![ArrayLength::Fixed(BigInt::from(3))],
            )),
        ),
    );
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PushStorage {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: 101,
                value: Some(num_literal!(1, 32)),
                storage: identifier(3)
            }
        ),
        "uint32 %temp.ssa_ir.101 = push_storage storage_ptr<uint32[3]>(%temp.ssa_ir.3) uint32(1);"
    );
}

#[test]
fn test_stringfy_pop_storage_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 123, Type::Uint(32));
    set_tmp(
        &mut v,
        3,
        Type::StoragePtr(
            false,
            Box::new(Type::Array(
                Box::new(Type::Uint(32)),
                vec![ArrayLength::Fixed(BigInt::from(3))],
            )),
        ),
    );
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PopStorage {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: Some(123),
                storage: identifier(3)
            }
        ),
        "uint32 %temp.ssa_ir.123 = pop_storage storage_ptr<uint32[3]>(%temp.ssa_ir.3);"
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PopStorage {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: None,
                storage: identifier(3)
            }
        ),
        "pop_storage storage_ptr<uint32[3]>(%temp.ssa_ir.3);"
    )
}

#[test]
fn test_stringfy_call_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Uint(8));
    set_tmp(&mut v, 2, Type::Uint(64));
    set_tmp(&mut v, 3, Type::Uint(8));
    set_tmp(&mut v, 133, Type::Uint(64));
    set_tmp(
        &mut v,
        123,
        Type::Ptr(Box::new(Type::Function {
            params: vec![Type::Uint(8), Type::Uint(64), Type::Uint(64)],
            returns: vec![Type::Uint(8), Type::Uint(64), Type::Uint(8)],
        })),
    );
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Call {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: vec![1, 2, 3],
                call: InternalCallTy::Builtin { ast_func_no: 123 },
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call builtin#123(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Call {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: vec![1, 2, 3],
                call: InternalCallTy::Dynamic(identifier(123)),
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call ptr<function (uint8, uint64, uint64) returns (uint8, uint64, uint8)>(%temp.ssa_ir.123)(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Call {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: vec![1, 2, 3],
                call: InternalCallTy::Static { cfg_no: 123 },
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call function#123(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );
}

//  ExternalCall
#[test]
fn test_stringfy_external_call_insn() {
    let mut v = new_vartable();

    // success
    set_tmp(&mut v, 1, Type::Bool);
    // payload
    set_tmp(&mut v, 3, Type::Bytes(32));
    // value
    set_tmp(&mut v, 4, Type::Uint(64));
    // gas
    set_tmp(&mut v, 7, Type::Uint(64));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ExternalCall {
                success: Some(1),
                address: None,
                payload: identifier(3),
                value: identifier(4),
                accounts: solang::sema::ast::ExternalCallAccounts::AbsentArgument,
                seeds: None,
                gas: identifier(7),
                callty: CallTy::Regular,
                contract_function_no: None,
                flags: None,
                loc: Loc::Codegen,
            }
        ),
        "bool %temp.ssa_ir.1 = call_ext [regular] address:_ payload:bytes32(%temp.ssa_ir.3) value:uint64(%temp.ssa_ir.4) gas:uint64(%temp.ssa_ir.7) accounts:absent seeds:_ contract_no:_, function_no:_ flags:_;"
    );
}

#[test]
fn test_stringfy_print_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 3, Type::Uint(8));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Print {
                loc: /*missing from cfg*/ Loc::Codegen,
                operand: identifier(3)
            }
        ),
        "print uint8(%temp.ssa_ir.3);"
    );
}

#[test]
fn test_stringfy_memcopy_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 3, Type::Bytes(32));
    set_tmp(&mut v, 4, Type::Bytes(16));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::MemCopy {
                loc: /*missing from cfg*/ Loc::Codegen,
                src: identifier(3),
                dest: identifier(4),
                bytes: num_literal!(16)
            }
        ),
        "memcopy bytes32(%temp.ssa_ir.3) to bytes16(%temp.ssa_ir.4) for uint8(16) bytes;"
    )
}

#[test]
fn test_stringfy_value_transfer_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bool);
    set_tmp(
        &mut v,
        2,
        Type::Array(
            Box::new(Type::Uint(8)),
            vec![ArrayLength::Fixed(BigInt::from(32))],
        ),
    );
    set_tmp(&mut v, 3, Type::Uint(8));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ValueTransfer {
                loc: /*missing from cfg*/ Loc::Codegen,
                success: Some(1),
                address: identifier(2),
                value: identifier(3),
            }
        ),
        "bool %temp.ssa_ir.1 = value_transfer uint8(%temp.ssa_ir.3) to uint8[32](%temp.ssa_ir.2);"
    );
}

#[test]
fn test_stringfy_selfdestruct_insn() {
    let mut v = new_vartable();

    set_tmp(
        &mut v,
        3,
        Type::Ptr(Box::new(Type::Struct(StructType::UserDefined(0)))),
    );
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::SelfDestruct {
                loc: /*missing from cfg*/ Loc::Codegen,
                recipient: identifier(3)
            }
        ),
        "self_destruct ptr<struct.0>(%temp.ssa_ir.3);"
    )
}

#[test]
fn test_stringfy_emit_event_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Bytes(32));
    set_tmp(&mut v, 2, Type::Bytes(32));
    set_tmp(&mut v, 3, Type::Bytes(32));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::EmitEvent {
                loc: /*missing from cfg*/ Loc::Codegen,
                event_no: 13,
                topics: vec![identifier(1), identifier(2)],
                data: identifier(3)
            }
        ),
        "emit event#13 to topics[bytes32(%temp.ssa_ir.1), bytes32(%temp.ssa_ir.2)], data: bytes32(%temp.ssa_ir.3);"
    )
}

#[test]
fn test_stringfy_branch_insn() {
    assert_eq!(
        stringfy_insn!(
            &new_printer(&new_vartable()),
            &Instruction::Branch {
                loc: /*missing from cfg*/ Loc::Codegen,
                block: 3
            }
        ),
        "br block#3;"
    )
}

#[test]
fn test_stringfy_branch_cond_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 3, Type::Bool);
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::BranchCond {
                loc: /*missing from cfg*/ Loc::Codegen,
                cond: identifier(3),
                true_block: 5,
                false_block: 6
            }
        ),
        "cbr bool(%temp.ssa_ir.3) block#5 else block#6;"
    )
}

#[test]
fn test_stringfy_switch_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Uint(8));
    set_tmp(&mut v, 4, Type::Uint(8));
    set_tmp(&mut v, 5, Type::Uint(8));
    set_tmp(&mut v, 6, Type::Uint(8));
    let printer = new_printer(&v);
    let s = stringfy_insn!(
        &printer,
        &Instruction::Switch {
            loc: /*missing from cfg*/ Loc::Codegen,
            cond: identifier(1),
            cases: vec![
                (identifier(4), 11),
                (identifier(5), 12),
                (identifier(6), 13),
            ],
            default: 14,
        }
    );
    assert_eq!(
        s,
        r#"switch uint8(%temp.ssa_ir.1):
    case:    uint8(%temp.ssa_ir.4) => block#11, 
    case:    uint8(%temp.ssa_ir.5) => block#12, 
    case:    uint8(%temp.ssa_ir.6) => block#13
    default: block#14;"#
    )
}

#[test]
fn test_stringfy_return_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Uint(8));
    set_tmp(&mut v, 2, Type::Bytes(32));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Return {
                loc: /*missing from cfg*/ Loc::Codegen,
                value: vec![identifier(1), identifier(2)]
            }
        ),
        "return uint8(%temp.ssa_ir.1), bytes32(%temp.ssa_ir.2);"
    )
}

#[test]
fn test_stringfy_assert_failure_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 3, Type::Bytes(32));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::AssertFailure {
                loc: /*missing from cfg*/ Loc::Codegen,
                encoded_args: Some(identifier(3))
            }
        ),
        "assert_failure bytes32(%temp.ssa_ir.3);"
    );

    assert_eq!(
        stringfy_insn!(
            &new_printer(&new_vartable()),
            &Instruction::AssertFailure {
                loc: /*missing from cfg*/ Loc::Codegen,
                encoded_args: None
            }
        ),
        "assert_failure;"
    )
}

#[test]
fn test_stringfy_phi_insn() {
    let mut v = new_vartable();

    set_tmp(&mut v, 1, Type::Uint(8));
    set_tmp(&mut v, 2, Type::Uint(8));
    set_tmp(&mut v, 12, Type::Uint(8));
    let printer = new_printer(&v);
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Phi {
                loc: /*missing from cfg*/ Loc::Codegen,
                res: 12,
                vars: vec![
                    PhiInput::new(identifier(1), 13),
                    PhiInput::new(identifier(2), 14)
                ],
            }
        ),
        "uint8 %temp.ssa_ir.12 = phi [uint8(%temp.ssa_ir.1), block#13], [uint8(%temp.ssa_ir.2), block#14];"
    )
}
