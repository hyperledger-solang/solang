use crate::num_literal;
use crate::ssa_ir_tests::helpers::{identifier, num_literal};
use solang::codegen::cfg;
use solang::sema::ast::CallTy;
use solang::ssa_ir::expr::{BinaryOperator, Expr};
use solang::ssa_ir::insn::Insn;
use solang::ssa_ir::ssa_type::{InternalCallTy, PhiInput};
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

// PushMemory
#[test]
fn test_stringfy_push_memory_insn() {
    assert_eq!(
        Insn::PushMemory {
            res: 101,
            array: 3,
            value: num_literal!(1, 32),
        }
        .to_string(),
        "%101 = push_mem %3 uint32(1);"
    );
}

#[test]
fn test_stringfy_pop_memory_insn() {
    assert_eq!(
        Insn::PopMemory {
            res: 101,
            array: 3,
            loc: Loc::Codegen,
        }
        .to_string(),
        "%101 = pop_mem %3;"
    );
}

// Constructor
#[test]
fn test_stringfy_constructor_insn() {
    assert_eq!(
        Insn::Constructor {
            success: Some(1),
            res: 13,
            contract_no: 0,
            constructor_no: Some(2),
            encoded_args: identifier(4),
            value: Some(num_literal!(5)),
            gas: num_literal!(300),
            salt: Some(num_literal!(22)),
            address: Some(identifier(6)),
            seeds: Some(identifier(7)),
            accounts: Some(identifier(8)),
            loc: Loc::Codegen
        }
        .to_string(),
        "%13, %1 = constructor(no: 2, contract_no:0) salt:uint8(22) value:uint8(5) gas:uint8(300) address:%6 seeds:%7 encoded-buffer:%4 accounts:%8;"
    );
}

// LoadStorage
#[test]
fn test_stringfy_load_storage_insn() {
    // "%{} = load_storage slot({}) ty:{};"
    assert_eq!(
        Insn::LoadStorage {
            res: 101,
            storage: identifier(3)
        }
        .to_string(),
        "%101 = load_storage %3;"
    );

    assert_eq!(
        Insn::LoadStorage {
            res: 101,
            storage: identifier(3)
        }
        .to_string(),
        "%101 = load_storage %3;"
    );

    assert_eq!(
        Insn::LoadStorage {
            res: 101,
            storage: identifier(3)
        }
        .to_string(),
        "%101 = load_storage %3;"
    );
}

#[test]
fn test_stringfy_clear_storage_insn() {
    assert_eq!(
        Insn::ClearStorage {
            storage: identifier(1)
        }
        .to_string(),
        "clear_storage %1;"
    );
}

#[test]
fn test_stringfy_set_storage_insn() {
    assert_eq!(
        Insn::SetStorage {
            value: num_literal(13445566, false, 256),
            storage: identifier(1)
        }
        .to_string(),
        "set_storage %1 uint256(13445566);"
    );
}

#[test]
fn test_stringfy_set_storage_bytes_insn() {
    // set_storage_bytes {} offset:{} value:{}
    assert_eq!(
        Insn::SetStorageBytes {
            value: identifier(1),
            storage: identifier(2),
            offset: num_literal!(3)
        }
        .to_string(),
        "set_storage_bytes %2 offset:uint8(3) value:%1;"
    );
}

#[test]
fn test_stringfy_push_storage_insn() {
    assert_eq!(
        Insn::PushStorage {
            res: 101,
            value: Some(num_literal!(1, 32)),
            storage: identifier(3)
        }
        .to_string(),
        "%101 = push_storage %3 uint32(1);"
    );
}

#[test]
fn test_stringfy_pop_storage_insn() {
    assert_eq!(
        Insn::PopStorage {
            res: Some(123),
            storage: identifier(3)
        }
        .to_string(),
        "%123 = pop_storage %3;"
    );

    assert_eq!(
        Insn::PopStorage {
            res: None,
            storage: identifier(3)
        }
        .to_string(),
        "pop_storage %3;"
    )
}

#[test]
fn test_stringfy_call_insn() {
    assert_eq!(
        Insn::Call {
            res: vec![1, 2, 3],
            call: InternalCallTy::Builtin { ast_func_no: 123 },
            args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
        }
        .to_string(),
        "%1, %2, %3 = call builtin#123(uint8(3), %133, uint64(6));"
    );

    assert_eq!(
        Insn::Call {
            res: vec![1, 2, 3],
            call: InternalCallTy::Dynamic(identifier(123)),
            args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
        }
        .to_string(),
        "%1, %2, %3 = call %123(uint8(3), %133, uint64(6));"
    );

    assert_eq!(
        Insn::Call {
            res: vec![1, 2, 3],
            call: InternalCallTy::Static { cfg_no: 123 },
            args: vec![num_literal!(3), identifier(133), num_literal!(6, 64)],
        }
        .to_string(),
        "%1, %2, %3 = call function#123(uint8(3), %133, uint64(6));"
    );
}

#[test]
fn test_stringfy_print_insn() {
    assert_eq!(
        Insn::Print {
            operand: identifier(3)
        }
        .to_string(),
        "print %3;"
    );
}

#[test]
fn test_stringfy_memcopy_insn() {
    assert_eq!(
        Insn::MemCopy {
            src: identifier(3),
            dest: identifier(4),
            bytes: num_literal!(11)
        }
        .to_string(),
        "memcopy %3 to %4 for uint8(11) bytes;"
    )
}

#[test]
fn test_stringfy_external_call_insn() {
    assert_eq!(
        Insn::ExternalCall {
            loc: Loc::Codegen,
            success: Some(1),
            address: Some(identifier(2)),
            accounts: Some(identifier(3)),
            seeds: Some(identifier(4)),
            payload: identifier(5),
            value: identifier(6),
            gas: num_literal!(120),
            callty: CallTy::Regular,
            contract_function_no: None,
            flags: Some(identifier(7)),
        }
        .to_string(),
        "%1 = call_ext [regular] address:%2 payload:%5 value:%6 gas:uint8(120) accounts:%3 seeds:%4 _ flags:%7;"
    );

    assert_eq!(
        Insn::ExternalCall {
            loc: Loc::Codegen,
            success: None,
            address: Some(identifier(2)),
            accounts: Some(identifier(3)),
            seeds: Some(identifier(4)),
            payload: identifier(5),
            value: identifier(6),
            gas: num_literal!(120),
            callty: CallTy::Delegate,
            contract_function_no: None,
            flags: Some(identifier(7)),
        }
        .to_string(),
        "call_ext [delegate] address:%2 payload:%5 value:%6 gas:uint8(120) accounts:%3 seeds:%4 _ flags:%7;"
    );

    assert_eq!(
        Insn::ExternalCall {
            loc: Loc::Codegen,
            success: None,
            address: Some(identifier(2)),
            accounts: Some(identifier(3)),
            seeds: None,
            payload: identifier(5),
            value: identifier(6),
            gas: num_literal!(120),
            callty: CallTy::Static,
            contract_function_no: None,
            flags: Some(identifier(7)),
        }
        .to_string(),
        "call_ext [static] address:%2 payload:%5 value:%6 gas:uint8(120) accounts:%3 _ _ flags:%7;"
    );
}

#[test]
fn test_stringfy_value_transfer_insn() {
    assert_eq!(
        Insn::ValueTransfer {
            success: Some(1),
            address: identifier(2),
            value: identifier(3),
        }
        .to_string(),
        "%1 = transfer %3 to %2;"
    );
}

#[test]
fn test_stringfy_selfdestruct_insn() {
    assert_eq!(
        Insn::SelfDestruct {
            recipient: identifier(3)
        }
        .to_string(),
        "self_destruct %3;"
    )
}

#[test]
fn test_stringfy_emit_event_insn() {
    // emit event#{} to topics[{}], data: {};
    assert_eq!(
        Insn::EmitEvent {
            event_no: 13,
            topics: vec![identifier(1), identifier(2)],
            data: identifier(3)
        }
        .to_string(),
        "emit event#13 to topics[%1, %2], data: %3;"
    )
}

#[test]
fn test_stringfy_write_buffer_insn() {
    assert_eq!(
        Insn::WriteBuffer {
            buf: identifier(1),
            offset: num_literal!(11),
            value: identifier(2)
        }
        .to_string(),
        "write_buf %1 offset:uint8(11) value:%2;"
    )
}

#[test]
fn test_stringfy_branch_insn() {
    assert_eq!(Insn::Branch { block: 3 }.to_string(), "br block#3;")
}

#[test]
fn test_stringfy_branch_cond_insn() {
    // cbr {} block#{} else block#{};
    assert_eq!(
        Insn::BranchCond {
            cond: identifier(3),
            true_block: 5,
            false_block: 6
        }
        .to_string(),
        "cbr %3 block#5 else block#6;"
    )
}

#[test]
fn test_stringfy_switch_insn() {
    assert_eq!(
        Insn::Switch {
            cond: identifier(1),
            cases: vec![
                (identifier(4), 11),
                (identifier(5), 12),
                (identifier(6), 13),
            ],
            default: 14,
        }
        .to_string(),
        "switch %1 cases: [%4 => block#11, %5 => block#12, %6 => block#13] default: block#14;"
    )
}

#[test]
fn test_stringfy_return_insn() {
    assert_eq!(
        Insn::Return {
            value: vec![identifier(1), identifier(2), identifier(3)]
        }
        .to_string(),
        "return %1, %2, %3;"
    )
}

#[test]
fn test_stringfy_assert_failure_insn() {
    assert_eq!(
        Insn::AssertFailure {
            encoded_args: Some(identifier(3))
        }
        .to_string(),
        "assert_failure %3;"
    );

    assert_eq!(
        Insn::AssertFailure { encoded_args: None }.to_string(),
        "assert_failure;"
    )
}

#[test]
fn test_stringfy_unimplemented_insn() {
    assert_eq!(
        Insn::Unimplemented { reachable: true }.to_string(),
        "unimplemented: reachable;"
    );

    assert_eq!(
        Insn::Unimplemented { reachable: false }.to_string(),
        "unimplemented: unreachable;"
    )
}

#[test]
fn test_stringfy_phi_insn() {
    assert_eq!(
        Insn::Phi {
            res: 12,
            vars: vec![
                PhiInput::new(identifier(1), 13),
                PhiInput::new(identifier(2), 14)
            ],
        }
        .to_string(),
        "%12 = phi [%1, block#13], [%2, block#14];"
    )
}
