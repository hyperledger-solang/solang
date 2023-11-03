// SPDX-License-Identifier: Apache-2.0

use crate::ssa_ir_tests::helpers::{identifier, num_literal};
use crate::{num_literal, stringfy_insn};
use indexmap::IndexMap;
use num_bigint::BigInt;
use solang::codegen::cfg;
use solang::sema::ast::{ArrayLength, CallTy};
use solang::ssa_ir::expressions::{BinaryOperator, Expression};
use solang::ssa_ir::instructions::Instruction;
use solang::ssa_ir::printer::Printer;
use solang::ssa_ir::ssa_type::{InternalCallTy, PhiInput, StructType, Type};
use solang::ssa_ir::vartable::Vartable;
use solang_parser::pt::Loc;

fn new_printer() -> Printer {
    let t = Vartable {
        vars: IndexMap::new(),
        args: IndexMap::new(),
        next_id: 0,
    };
    Printer {
        vartable: Box::new(t),
    }
}

#[test]
fn test_stringfy_nop_insn() {
    assert_eq!(stringfy_insn!(&new_printer(), &Instruction::Nop), "nop;");
}

// ReturnData
#[test]
fn test_stringfy_returndata_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(0, &Type::Bytes(1));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ReturnData {
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
            &new_printer(),
            &Instruction::ReturnCode {
                code: cfg::ReturnCode::AbiEncodingInvalid,
            }
        ),
        "return_code \"abi encoding invalid\";"
    );

    assert_eq!(
        stringfy_insn!(
            &new_printer(),
            &Instruction::ReturnCode {
                code: cfg::ReturnCode::AccountDataTooSmall,
            }
        ),
        "return_code \"account data too small\";"
    );
}

// Set
#[test]
fn test_stringfy_set_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(121, &Type::Uint(8));
    printer.set_tmp_var(122, &Type::Uint(8));

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
    let mut printer = new_printer();
    printer.set_tmp_var(0, &Type::Ptr(Box::new(Type::Uint(8))));
    printer.set_tmp_var(1, &Type::Uint(8));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Store {
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
    let mut printer = new_printer();
    printer.set_tmp_var(
        3,
        &Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(32)),
            vec![ArrayLength::Fixed(BigInt::from(3))],
        ))),
    );
    printer.set_tmp_var(101, &Type::Uint(32));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PushMemory {
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
    let mut printer = new_printer();
    printer.set_tmp_var(
        3,
        &Type::Ptr(Box::new(Type::Array(
            Box::new(Type::Uint(32)),
            vec![ArrayLength::Fixed(BigInt::from(3))],
        ))),
    );
    printer.set_tmp_var(101, &Type::Uint(32));

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
    let mut printer = new_printer();
    printer.set_tmp_var(101, &Type::Uint(32));
    printer.set_tmp_var(3, &Type::StoragePtr(false, Box::new(Type::Uint(32))));

    // "%{} = load_storage slot({}) ty:{};"
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::LoadStorage {
                res: 101,
                storage: identifier(3)
            }
        ),
        "uint32 %temp.ssa_ir.101 = load_storage storage_ptr<uint32>(%temp.ssa_ir.3);"
    );
}

#[test]
fn test_stringfy_clear_storage_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(3, &Type::StoragePtr(false, Box::new(Type::Uint(32))));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ClearStorage {
                storage: identifier(3)
            }
        ),
        "clear_storage storage_ptr<uint32>(%temp.ssa_ir.3);"
    );
}

#[test]
fn test_stringfy_set_storage_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::StoragePtr(false, Box::new(Type::Uint(256))));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::SetStorage {
                value: num_literal(13445566, false, 256),
                storage: identifier(1)
            }
        ),
        "set_storage storage_ptr<uint256>(%temp.ssa_ir.1) uint256(13445566);"
    );
}

#[test]
fn test_stringfy_set_storage_bytes_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Bytes(32));
    printer.set_tmp_var(2, &Type::StoragePtr(false, Box::new(Type::Bytes(32))));

    // set_storage_bytes {} offset:{} value:{}
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::SetStorageBytes {
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
    let mut printer = new_printer();
    printer.set_tmp_var(101, &Type::Uint(32));
    printer.set_tmp_var(
        3,
        &Type::StoragePtr(
            false,
            Box::new(Type::Array(
                Box::new(Type::Uint(32)),
                vec![ArrayLength::Fixed(BigInt::from(3))],
            )),
        ),
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PushStorage {
                res: 101,
                value: Some(num_literal!(1, 32)),
                storage: identifier(3)
            }
        ),
        // "%101 = push_storage %3 uint32(1);"
        "uint32 %temp.ssa_ir.101 = push_storage storage_ptr<uint32[3]>(%temp.ssa_ir.3) uint32(1);"
    );
}

#[test]
fn test_stringfy_pop_storage_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(123, &Type::Uint(32));
    printer.set_tmp_var(
        3,
        &Type::StoragePtr(
            false,
            Box::new(Type::Array(
                Box::new(Type::Uint(32)),
                vec![ArrayLength::Fixed(BigInt::from(3))],
            )),
        ),
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PopStorage {
                res: Some(123),
                storage: identifier(3)
            }
        ),
        // "%123 = pop_storage %3;"
        "uint32 %temp.ssa_ir.123 = pop_storage storage_ptr<uint32[3]>(%temp.ssa_ir.3);"
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::PopStorage {
                res: None,
                storage: identifier(3)
            }
        ),
        // "pop_storage %3;"
        "pop_storage storage_ptr<uint32[3]>(%temp.ssa_ir.3);"
    )
}

#[test]
fn test_stringfy_call_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Uint(8));
    printer.set_tmp_var(2, &Type::Uint(64));
    printer.set_tmp_var(3, &Type::Uint(8));
    printer.set_tmp_var(133, &Type::Uint(64));
    printer.set_tmp_var(
        123,
        &Type::Ptr(Box::new(Type::Function {
            params: vec![Type::Uint(8), Type::Uint(64), Type::Uint(64)],
            returns: vec![Type::Uint(8), Type::Uint(64), Type::Uint(8)],
        })),
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Call {
                res: vec![1, 2, 3],
                call: InternalCallTy::Builtin { ast_func_no: 123 },
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        // "%1, %2, %3 = call builtin#123(uint8(3), %133, uint64(6));"
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call builtin#123(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Call {
                res: vec![1, 2, 3],
                call: InternalCallTy::Dynamic(identifier(123)),
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        // "%1, %2, %3 = call %123(uint8(3), %133, uint64(6));"
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call ptr<fn(uint8, uint64, uint64) -> (uint8, uint64, uint8)>(%temp.ssa_ir.123)(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Call {
                res: vec![1, 2, 3],
                call: InternalCallTy::Static { cfg_no: 123 },
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        // "%1, %2, %3 = call function#123(uint8(3), %133, uint64(6));"
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call function#123(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );
}

//  ExternalCall
#[test]
fn test_stringfy_external_call_insn() {
    // Insn::ExternalCall {
    //     success,
    //     address,
    //     payload,
    //     value,
    //     accounts,
    //     seeds,
    //     gas,
    //     callty,
    //     contract_function_no,
    //     flags,
    // }

    // {} = call_ext ty:{} address:{} payload:{} value:{} gas:{} accounts:{} seeds:{} contract_no:{}, function_no:{} flags:{};

    let mut printer = new_printer();
    // success
    printer.set_tmp_var(1, &Type::Bool);
    // payload
    printer.set_tmp_var(3, &Type::Bytes(32));
    // value
    printer.set_tmp_var(4, &Type::Uint(64));
    // gas
    printer.set_tmp_var(7, &Type::Uint(64));

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
    let mut printer = new_printer();
    printer.set_tmp_var(3, &Type::Uint(8));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Print {
                operand: identifier(3)
            }
        ),
        "print uint8(%temp.ssa_ir.3);"
    );
}

#[test]
fn test_stringfy_memcopy_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(3, &Type::Bytes(32));
    printer.set_tmp_var(4, &Type::Bytes(16));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::MemCopy {
                src: identifier(3),
                dest: identifier(4),
                bytes: num_literal!(16)
            }
        ),
        // "memcopy %3 to %4 for uint8(16) bytes;"
        "memcopy bytes32(%temp.ssa_ir.3) to bytes16(%temp.ssa_ir.4) for uint8(16) bytes;"
    )
}

#[test]
fn test_stringfy_value_transfer_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Bool);
    printer.set_tmp_var(
        2,
        &Type::Array(
            Box::new(Type::Uint(8)),
            vec![ArrayLength::Fixed(BigInt::from(32))],
        ),
    );
    printer.set_tmp_var(3, &Type::Uint(8));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::ValueTransfer {
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
    let mut printer = new_printer();
    printer.set_tmp_var(
        3,
        &Type::Ptr(Box::new(Type::Struct(StructType::UserDefined(0)))),
    );

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::SelfDestruct {
                recipient: identifier(3)
            }
        ),
        "self_destruct ptr<struct.0>(%temp.ssa_ir.3);"
    )
}

#[test]
fn test_stringfy_emit_event_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Bytes(32));
    printer.set_tmp_var(2, &Type::Bytes(32));
    printer.set_tmp_var(3, &Type::Bytes(32));

    // emit event#{} to topics[{}], data: {};
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::EmitEvent {
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
        stringfy_insn!(&new_printer(), &Instruction::Branch { block: 3 }),
        "br block#3;"
    )
}

#[test]
fn test_stringfy_branch_cond_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(3, &Type::Bool);

    // cbr {} block#{} else block#{};
    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::BranchCond {
                cond: identifier(3),
                true_block: 5,
                false_block: 6
            }
        ),
        // "cbr %3 block#5 else block#6;"
        "cbr bool(%temp.ssa_ir.3) block#5 else block#6;"
    )
}

#[test]
fn test_stringfy_switch_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Uint(8));
    printer.set_tmp_var(4, &Type::Uint(8));
    printer.set_tmp_var(5, &Type::Uint(8));
    printer.set_tmp_var(6, &Type::Uint(8));

    let s = stringfy_insn!(
        &printer,
        &Instruction::Switch {
            cond: identifier(1),
            cases: vec![
                (identifier(4), 11),
                (identifier(5), 12),
                (identifier(6), 13),
            ],
            default: 14,
        }
    );
    // println!("{}", s);
    assert_eq!(
        s,
        // "switch %1 cases: [%4 => block#11, %5 => block#12, %6 => block#13] default: block#14;"
        r#"switch uint8(%temp.ssa_ir.1):
    case:    uint8(%temp.ssa_ir.4) => block#11, 
    case:    uint8(%temp.ssa_ir.5) => block#12, 
    case:    uint8(%temp.ssa_ir.6) => block#13
    default: block#14;"#
    )
}

#[test]
fn test_stringfy_return_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Uint(8));
    printer.set_tmp_var(2, &Type::Bytes(32));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Return {
                value: vec![identifier(1), identifier(2)]
            }
        ),
        "return uint8(%temp.ssa_ir.1), bytes32(%temp.ssa_ir.2);"
    )
}

#[test]
fn test_stringfy_assert_failure_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(3, &Type::Bytes(32));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::AssertFailure {
                encoded_args: Some(identifier(3))
            }
        ),
        "assert_failure bytes32(%temp.ssa_ir.3);"
    );

    assert_eq!(
        stringfy_insn!(
            &new_printer(),
            &Instruction::AssertFailure { encoded_args: None }
        ),
        "assert_failure;"
    )
}

#[test]
fn test_stringfy_unimplemented_insn() {
    assert_eq!(
        stringfy_insn!(
            &new_printer(),
            &Instruction::Unimplemented { reachable: true }
        ),
        "unimplemented: reachable;"
    );

    assert_eq!(
        stringfy_insn!(
            &new_printer(),
            &Instruction::Unimplemented { reachable: false }
        ),
        "unimplemented: unreachable;"
    )
}

#[test]
fn test_stringfy_phi_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(1, &Type::Uint(8));
    printer.set_tmp_var(2, &Type::Uint(8));
    printer.set_tmp_var(12, &Type::Uint(8));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Instruction::Phi {
                res: 12,
                vars: vec![
                    PhiInput::new(identifier(1), 13),
                    PhiInput::new(identifier(2), 14)
                ],
            }
        ),
        // "%12 = phi [%1, block#13], [%2, block#14];"
        "uint8 %temp.ssa_ir.12 = phi [uint8(%temp.ssa_ir.1), block#13], [uint8(%temp.ssa_ir.2), block#14];"
    )
}
