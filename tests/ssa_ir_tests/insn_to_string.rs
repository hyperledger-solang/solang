use crate::num_literal;
use crate::ssa_ir_tests::helpers::{identifier, num_literal};
use indexmap::IndexMap;
use num_bigint::BigInt;
use solang::codegen::cfg;
use solang::sema::ast::ArrayLength;
use solang::ssa_ir::expr::{BinaryOperator, Expr};
use solang::ssa_ir::insn::Insn;
use solang::ssa_ir::printer::Printer;
use solang::ssa_ir::ssa_type::{InternalCallTy, PhiInput, StructType, Type};
use solang::ssa_ir::vartable::Vartable;
use solang::stringfy_insn;
use solang_parser::pt::Loc;

fn new_printer() -> Printer {
    let t = Vartable {
        vars: IndexMap::new(),
        next_id: 0,
    };
    Printer {
        vartable: Box::new(t),
    }
}

#[test]
fn test_stringfy_nop_insn() {
    assert_eq!(stringfy_insn!(&new_printer(), &Insn::Nop), "nop;");
}

// ReturnData
#[test]
fn test_stringfy_returndata_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(0, &Type::Bytes(1));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Insn::ReturnData {
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
            &Insn::ReturnCode {
                code: cfg::ReturnCode::AbiEncodingInvalid,
            }
        ),
        "return_code \"abi encoding invalid\";"
    );

    assert_eq!(
        stringfy_insn!(
            &new_printer(),
            &Insn::ReturnCode {
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
            &Insn::Set {
                loc: Loc::Codegen,
                res: 122,
                expr: Expr::BinaryExpr {
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
            &Insn::Store {
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
            &Insn::Store {
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
            &Insn::PushMemory {
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
            &Insn::PopMemory {
                res: 101,
                array: 3,
                loc: Loc::Codegen,
            }
        ),
        "uint32 %temp.ssa_ir.101 = pop_mem ptr<uint32[3]>(%temp.ssa_ir.3);"
    );
}

// Constructor
// #[test]
// fn test_stringfy_constructor_insn() {
//     assert_eq!(
//         stringfy_insn!(&new_printer(), &Insn::Constructor {
//             success: Some(1),
//             res: 13,
//             contract_no: 0,
//             constructor_no: Some(2),
//             encoded_args: identifier(4),
//             value: Some(num_literal!(5)),
//             gas: num_literal!(300),
//             salt: Some(num_literal!(22)),
//             address: Some(identifier(6)),
//             seeds: Some(identifier(7)),
//             accounts: Some(identifier(8)),
//             loc: Loc::Codegen
//         }
//         ),
//         "%13, %1 = constructor(no: 2, contract_no:0) salt:uint8(22) value:uint8(5) gas:uint8(300) address:%6 seeds:%7 encoded-buffer:%4 accounts:%8;"
//     );
// }

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
            &Insn::LoadStorage {
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
            &Insn::ClearStorage {
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
            &Insn::SetStorage {
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
            &Insn::SetStorageBytes {
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
            &Insn::PushStorage {
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
            &Insn::PopStorage {
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
            &Insn::PopStorage {
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
            &Insn::Call {
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
            &Insn::Call {
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
            &Insn::Call {
                res: vec![1, 2, 3],
                call: InternalCallTy::Static { cfg_no: 123 },
                args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
            }
        ),
        // "%1, %2, %3 = call function#123(uint8(3), %133, uint64(6));"
        "uint8 %temp.ssa_ir.1, uint64 %temp.ssa_ir.2, uint8 %temp.ssa_ir.3 = call function#123(uint8(3), uint64(%temp.ssa_ir.133), uint64(6));"
    );
}

#[test]
fn test_stringfy_print_insn() {
    let mut printer = new_printer();
    printer.set_tmp_var(3, &Type::Uint(8));

    assert_eq!(
        stringfy_insn!(
            &printer,
            &Insn::Print {
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
            &Insn::MemCopy {
                src: identifier(3),
                dest: identifier(4),
                bytes: num_literal!(16)
            }
        ),
        // "memcopy %3 to %4 for uint8(16) bytes;"
        "memcopy bytes32(%temp.ssa_ir.3) to bytes16(%temp.ssa_ir.4) for uint8(16) bytes;"
    )
}

// #[test]
// fn test_stringfy_external_call_insn() {
//     assert_eq!(
//         stringfy_insn!(&new_printer(), &Insn::ExternalCall {
//             loc: Loc::Codegen,
//             success: Some(1),
//             address: Some(identifier(2)),
//             accounts: Some(identifier(3)),
//             seeds: Some(identifier(4)),
//             payload: identifier(5),
//             value: identifier(6),
//             gas: num_literal!(120),
//             callty: CallTy::Regular,
//             contract_function_no: None,
//             flags: Some(identifier(7)),
//         }
//         ),
//         "%1 = call_ext [regular] address:%2 payload:%5 value:%6 gas:uint8(120) accounts:%3 seeds:%4 _ flags:%7;"
//     );

//     assert_eq!(
//         stringfy_insn!(&new_printer(), &Insn::ExternalCall {
//             loc: Loc::Codegen,
//             success: None,
//             address: Some(identifier(2)),
//             accounts: Some(identifier(3)),
//             seeds: Some(identifier(4)),
//             payload: identifier(5),
//             value: identifier(6),
//             gas: num_literal!(120),
//             callty: CallTy::Delegate,
//             contract_function_no: None,
//             flags: Some(identifier(7)),
//         }
//         ),
//         "call_ext [delegate] address:%2 payload:%5 value:%6 gas:uint8(120) accounts:%3 seeds:%4 _ flags:%7;"
//     );

//     assert_eq!(
//         stringfy_insn!(
//             &new_printer(),
//             &Insn::ExternalCall {
//                 loc: Loc::Codegen,
//                 success: None,
//                 address: Some(identifier(2)),
//                 accounts: Some(identifier(3)),
//                 seeds: None,
//                 payload: identifier(5),
//                 value: identifier(6),
//                 gas: num_literal!(120),
//                 callty: CallTy::Static,
//                 contract_function_no: None,
//                 flags: Some(identifier(7)),
//             }
//         ),
//         "call_ext [static] address:%2 payload:%5 value:%6 gas:uint8(120) accounts:%3 _ _ flags:%7;"
//     );
// }

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
            &Insn::ValueTransfer {
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
            &Insn::SelfDestruct {
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
            &Insn::EmitEvent {
                event_no: 13,
                topics: vec![identifier(1), identifier(2)],
                data: identifier(3)
            }
        ),
        "emit event#13 to topics[bytes32(%temp.ssa_ir.1), bytes32(%temp.ssa_ir.2)], data: bytes32(%temp.ssa_ir.3);"
    )
}

// #[test]
// fn test_stringfy_write_buffer_insn() {

//     assert_eq!(
//         stringfy_insn!(
//             &new_printer(),
//             &Insn::WriteBuffer {
//                 buf: identifier(1),
//                 offset: num_literal!(11),
//                 value: identifier(2)
//             }
//         ),
//         "write_buf %1 offset:uint8(11) value:%2;"
//     )
// }

#[test]
fn test_stringfy_branch_insn() {
    assert_eq!(
        stringfy_insn!(&new_printer(), &Insn::Branch { block: 3 }),
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
            &Insn::BranchCond {
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
        &Insn::Switch {
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
            &Insn::Return {
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
            &Insn::AssertFailure {
                encoded_args: Some(identifier(3))
            }
        ),
        "assert_failure bytes32(%temp.ssa_ir.3);"
    );

    assert_eq!(
        stringfy_insn!(&new_printer(), &Insn::AssertFailure { encoded_args: None }),
        "assert_failure;"
    )
}

#[test]
fn test_stringfy_unimplemented_insn() {
    assert_eq!(
        stringfy_insn!(&new_printer(), &Insn::Unimplemented { reachable: true }),
        "unimplemented: reachable;"
    );

    assert_eq!(
        stringfy_insn!(&new_printer(), &Insn::Unimplemented { reachable: false }),
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
            &Insn::Phi {
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
