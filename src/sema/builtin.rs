// SPDX-License-Identifier: Apache-2.0

use super::ast::{
    ArrayLength, Builtin, Diagnostic, Expression, File, Function, Namespace, Parameter, StructType,
    Symbol, Type,
};
use super::diagnostics::Diagnostics;
use super::eval::eval_const_number;
use super::expression::{expression, ExprContext, ResolveTo};
use super::symtable::Symtable;
use crate::sema::ast::RetrieveType;
use crate::Target;
use num_bigint::BigInt;
use num_traits::One;
use once_cell::sync::Lazy;
use solang_parser::pt::CodeLocation;
use solang_parser::pt::{self, Identifier};
use std::path::PathBuf;

pub struct Prototype {
    pub builtin: Builtin,
    pub namespace: Option<&'static str>,
    pub method: Option<Type>,
    pub name: &'static str,
    pub params: Vec<Type>,
    pub ret: Vec<Type>,
    pub target: Vec<Target>,
    pub doc: &'static str,
    // Can this function be called in constant context (e.g. hash functions)
    pub constant: bool,
}

// A list of all Solidity builtins functions
static BUILTIN_FUNCTIONS: Lazy<[Prototype; 27]> = Lazy::new(|| {
    [
        Prototype {
            builtin: Builtin::Assert,
            namespace: None,
            method: None,
            name: "assert",
            params: vec![Type::Bool],
            ret: vec![Type::Void],
            target: vec![],
            doc: "Abort execution if argument evaluates to false",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Print,
            namespace: None,
            method: None,
            name: "print",
            params: vec![Type::String],
            ret: vec![Type::Void],
            target: vec![],
            doc: "log string for debugging purposes. Runs on development chain only",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Require,
            namespace: None,
            method: None,
            name: "require",
            params: vec![Type::Bool],
            ret: vec![Type::Void],
            target: vec![],
            doc: "Abort execution if argument evaulates to false",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Require,
            namespace: None,
            method: None,
            name: "require",
            params: vec![Type::Bool, Type::String],
            ret: vec![Type::Void],
            target: vec![],
            doc: "Abort execution if argument evaulates to false. Report string when aborting",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Revert,
            namespace: None,
            method: None,
            name: "revert",
            params: vec![],
            ret: vec![Type::Unreachable],
            target: vec![],
            doc: "Revert execution",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Revert,
            namespace: None,
            method: None,
            name: "revert",
            params: vec![Type::String],
            ret: vec![Type::Unreachable],
            target: vec![],
            doc: "Revert execution and report string",
            constant: false,
        },
        Prototype {
            builtin: Builtin::SelfDestruct,
            namespace: None,
            method: None,
            name: "selfdestruct",
            params: vec![Type::Address(true)],
            ret: vec![Type::Unreachable],
            target: vec![Target::Ewasm, Target::default_substrate()],
            doc: "Destroys current account and deposits any remaining balance to address",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Keccak256,
            namespace: None,
            method: None,
            name: "keccak256",
            params: vec![Type::DynamicBytes],
            ret: vec![Type::Bytes(32)],
            target: vec![],
            doc: "Calculates keccak256 hash",
            constant: true,
        },
        Prototype {
            builtin: Builtin::Ripemd160,
            namespace: None,
            method: None,
            name: "ripemd160",
            params: vec![Type::DynamicBytes],
            ret: vec![Type::Bytes(20)],
            target: vec![],
            doc: "Calculates ripemd hash",
            constant: true,
        },
        Prototype {
            builtin: Builtin::Sha256,
            namespace: None,
            method: None,
            name: "sha256",
            params: vec![Type::DynamicBytes],
            ret: vec![Type::Bytes(32)],
            target: vec![],
            doc: "Calculates sha256 hash",
            constant: true,
        },
        Prototype {
            builtin: Builtin::Blake2_128,
            namespace: None,
            method: None,
            name: "blake2_128",
            params: vec![Type::DynamicBytes],
            ret: vec![Type::Bytes(16)],
            target: vec![Target::default_substrate()],
            doc: "Calculates blake2-128 hash",
            constant: true,
        },
        Prototype {
            builtin: Builtin::Blake2_256,
            namespace: None,
            method: None,
            name: "blake2_256",
            params: vec![Type::DynamicBytes],
            ret: vec![Type::Bytes(32)],
            target: vec![Target::default_substrate()],
            doc: "Calculates blake2-256 hash",
            constant: true,
        },
        Prototype {
            builtin: Builtin::Gasleft,
            namespace: None,
            method: None,
            name: "gasleft",
            params: vec![],
            ret: vec![Type::Uint(64)],
            target: vec![Target::default_substrate(), Target::Ewasm],
            doc: "Return remaining gas left in current call",
            constant: false,
        },
        Prototype {
            builtin: Builtin::BlockHash,
            namespace: None,
            method: None,
            name: "blockhash",
            params: vec![Type::Uint(64)],
            ret: vec![Type::Bytes(32)],
            target: vec![Target::Ewasm],
            doc: "Returns the block hash for given block number",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Random,
            namespace: None,
            method: None,
            name: "random",
            params: vec![Type::DynamicBytes],
            ret: vec![Type::Bytes(32)],
            target: vec![Target::default_substrate()],
            doc: "Returns deterministic random bytes",
            constant: false,
        },
        Prototype {
            builtin: Builtin::AbiDecode,
            namespace: Some("abi"),
            method: None,
            name: "decode",
            params: vec![Type::DynamicBytes],
            ret: vec![],
            target: vec![],
            doc: "Abi decode byte array with the given types",
            constant: false,
        },
        Prototype {
            builtin: Builtin::AbiEncode,
            namespace: Some("abi"),
            method: None,
            name: "encode",
            params: vec![],
            ret: vec![],
            target: vec![],
            doc: "Abi encode given arguments",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::AbiEncodePacked,
            namespace: Some("abi"),
            method: None,
            name: "encodePacked",
            params: vec![],
            ret: vec![],
            target: vec![],
            doc: "Abi encode given arguments using packed encoding",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::AbiEncodeWithSelector,
            namespace: Some("abi"),
            method: None,
            name: "encodeWithSelector",
            params: vec![Type::Bytes(4)],
            ret: vec![],
            target: vec![],
            doc: "Abi encode given arguments with selector",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::AbiEncodeWithSignature,
            namespace: Some("abi"),
            method: None,
            name: "encodeWithSignature",
            params: vec![Type::String],
            ret: vec![],
            target: vec![],
            doc: "Abi encode given arguments with function signature",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::AbiEncodeCall,
            namespace: Some("abi"),
            method: None,
            name: "encodeCall",
            params: vec![],
            ret: vec![],
            target: vec![],
            doc: "Abi encode given arguments with function signature",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::Gasprice,
            namespace: Some("tx"),
            method: None,
            name: "gasprice",
            params: vec![Type::Uint(64)],
            ret: vec![Type::Value],
            target: vec![],
            doc: "Calculate price of given gas units",
            constant: false,
        },
        Prototype {
            builtin: Builtin::MulMod,
            namespace: None,
            method: None,
            name: "mulmod",
            params: vec![Type::Uint(256), Type::Uint(256), Type::Uint(256)],
            ret: vec![Type::Uint(256)],
            target: vec![],
            doc: "Multiply first two arguments, and the modulo last argument. Does not overflow",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::AddMod,
            namespace: None,
            method: None,
            name: "addmod",
            params: vec![Type::Uint(256), Type::Uint(256), Type::Uint(256)],
            ret: vec![Type::Uint(256)],
            target: vec![],
            doc: "Add first two arguments, and the modulo last argument. Does not overflow",
            // it should be allowed in constant context, but we don't support that yet
            constant: false,
        },
        Prototype {
            builtin: Builtin::SignatureVerify,
            namespace: None,
            method: None,
            name: "signatureVerify",
            params: vec![Type::Address(false), Type::DynamicBytes, Type::DynamicBytes],
            ret: vec![Type::Bool],
            target: vec![Target::Solana],
            doc: "ed25519 signature verification",
            constant: false,
        },
        Prototype {
            builtin: Builtin::UserTypeWrap,
            namespace: None,
            method: Some(Type::UserType(0)),
            name: "wrap",
            params: vec![],
            ret: vec![Type::UserType(0)],
            target: vec![],
            doc: "wrap type into user defined type",
            constant: false,
        },
        Prototype {
            builtin: Builtin::UserTypeUnwrap,
            namespace: None,
            method: Some(Type::UserType(0)),
            name: "unwrap",
            params: vec![Type::UserType(0)],
            ret: vec![],
            target: vec![],
            doc: "unwrap user defined type",
            constant: false,
        },
    ]
});

// A list of all Solidity builtins variables
static BUILTIN_VARIABLE: Lazy<[Prototype; 16]> = Lazy::new(|| {
    [
        Prototype {
            builtin: Builtin::BlockCoinbase,
            namespace: Some("block"),
            method: None,
            name: "coinbase",
            params: vec![],
            ret: vec![Type::Address(true)],
            target: vec![Target::Ewasm],
            doc: "The address of the current block miner",
            constant: false,
        },
        Prototype {
            builtin: Builtin::BlockDifficulty,
            namespace: Some("block"),
            method: None,
            name: "difficulty",
            params: vec![],
            ret: vec![Type::Uint(256)],
            target: vec![Target::Ewasm],
            doc: "The difficulty for current block",
            constant: false,
        },
        Prototype {
            builtin: Builtin::GasLimit,
            namespace: Some("block"),
            method: None,
            name: "gaslimit",
            params: vec![],
            ret: vec![Type::Uint(64)],
            target: vec![Target::Ewasm],
            doc: "The gas limit",
            constant: false,
        },
        Prototype {
            builtin: Builtin::BlockNumber,
            namespace: Some("block"),
            method: None,
            name: "number",
            params: vec![],
            ret: vec![Type::Uint(64)],
            target: vec![],
            doc: "Current block number",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Slot,
            namespace: Some("block"),
            method: None,
            name: "slot",
            params: vec![],
            ret: vec![Type::Uint(64)],
            target: vec![Target::Solana],
            doc: "Current slot number",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ProgramId,
            namespace: Some("tx"),
            method: None,
            name: "program_id",
            params: vec![],
            ret: vec![Type::Address(false)],
            target: vec![Target::Solana],
            doc: "Program ID of currently executing program",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Timestamp,
            namespace: Some("block"),
            method: None,
            name: "timestamp",
            params: vec![],
            ret: vec![Type::Uint(64)],
            target: vec![],
            doc: "Current timestamp in unix epoch (seconds since 1970)",
            constant: false,
        },
        Prototype {
            builtin: Builtin::TombstoneDeposit,
            namespace: Some("block"),
            method: None,
            name: "tombstone_deposit",
            params: vec![],
            ret: vec![Type::Value],
            target: vec![Target::default_substrate()],
            doc: "Deposit required for a tombstone",
            constant: false,
        },
        Prototype {
            builtin: Builtin::MinimumBalance,
            namespace: Some("block"),
            method: None,
            name: "minimum_balance",
            params: vec![],
            ret: vec![Type::Value],
            target: vec![Target::default_substrate()],
            doc: "Minimum balance required for an account",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Calldata,
            namespace: Some("msg"),
            method: None,
            name: "data",
            params: vec![],
            ret: vec![Type::DynamicBytes],
            target: vec![],
            doc: "Raw input bytes to current call",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Sender,
            namespace: Some("msg"),
            method: None,
            name: "sender",
            params: vec![],
            ret: vec![Type::Address(true)],
            target: vec![],
            constant: false,
            doc: "Address of caller",
        },
        Prototype {
            builtin: Builtin::Signature,
            namespace: Some("msg"),
            method: None,
            name: "sig",
            params: vec![],
            ret: vec![Type::Bytes(4)],
            target: vec![],
            doc: "Function selector for current call",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Value,
            namespace: Some("msg"),
            method: None,
            name: "value",
            params: vec![],
            ret: vec![Type::Value],
            target: vec![],
            doc: "Value sent with current call",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Gasprice,
            namespace: Some("tx"),
            method: None,
            name: "gasprice",
            params: vec![],
            ret: vec![Type::Value],
            target: vec![Target::default_substrate(), Target::Ewasm],
            doc: "gas price for one gas unit",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Origin,
            namespace: Some("tx"),
            method: None,
            name: "origin",
            params: vec![],
            ret: vec![Type::Address(true)],
            target: vec![Target::Ewasm],
            doc: "Original address of sender current transaction",
            constant: false,
        },
        Prototype {
            builtin: Builtin::Accounts,
            namespace: Some("tx"),
            method: None,
            name: "accounts",
            params: vec![],
            ret: vec![Type::Array(
                Box::new(Type::Struct(StructType::AccountInfo)),
                vec![ArrayLength::Dynamic],
            )],
            target: vec![Target::Solana],
            doc: "Accounts passed into transaction",
            constant: false,
        },
    ]
});

// A list of all Solidity builtins methods
static BUILTIN_METHODS: Lazy<[Prototype; 25]> = Lazy::new(|| {
    [
        Prototype {
            builtin: Builtin::ReadInt8,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readInt8",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Int(8)],
            target: vec![],
            doc: "Reads a signed 8-bit integer from the specified offset",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadInt16LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readInt16LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Int(16)],
            target: vec![],
            doc: "Reads a signed 16-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadInt32LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readInt32LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Int(32)],
            target: vec![],
            doc: "Reads a signed 32-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadInt64LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readInt64LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Int(64)],
            target: vec![],
            doc: "Reads a signed 64-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadInt128LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readInt128LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Int(128)],
            target: vec![],
            doc: "Reads a signed 128-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadInt256LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readInt256LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Int(256)],
            target: vec![],
            doc: "Reads a signed 256-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadUint8,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readUint8",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Uint(8)],
            target: vec![],
            doc: "Reads an unsigned 8-bit integer from the specified offset",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadUint16LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readUint16LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Uint(16)],
            target: vec![],
            doc: "Reads an unsigned 16-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadUint32LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readUint32LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Uint(32)],
            target: vec![],
            doc: "Reads an unsigned 32-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadUint64LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readUint64LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Uint(64)],
            target: vec![],
            doc: "Reads an unsigned 64-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadUint128LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readUint128LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Uint(128)],
            target: vec![],
            doc: "Reads an unsigned 128-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadUint256LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readUint256LE",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Uint(256)],
            target: vec![],
            doc: "Reads an unsigned 256-bit integer from the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::ReadAddress,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "readAddress",
            params: vec![Type::Uint(32)],
            ret: vec![Type::Address(false)],
            target: vec![],
            doc: "Reads an address from the specified offset",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteInt8,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeInt8",
            params: vec![Type::Int(8), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes a signed 8-bit integer to the specified offset",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteInt16LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeInt16LE",
            params: vec![Type::Int(16), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes a signed 16-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteInt32LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeInt32LE",
            params: vec![Type::Int(32), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes a signed 32-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteInt64LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeInt64LE",
            params: vec![Type::Int(64), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes a signed 64-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteInt128LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeInt128LE",
            params: vec![Type::Int(128), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes a signed 128-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteInt256LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeInt256LE",
            params: vec![Type::Int(256), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes a signed 256-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteUint16LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeUint16LE",
            params: vec![Type::Uint(16), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes an unsigned 16-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteUint32LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeUint32LE",
            params: vec![Type::Uint(32), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes an unsigned 32-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteUint64LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeUint64LE",
            params: vec![Type::Uint(64), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes an unsigned 64-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteUint128LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeUint128LE",
            params: vec![Type::Uint(128), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes an unsigned 128-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteUint256LE,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeUint256LE",
            params: vec![Type::Uint(256), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes an unsigned 256-bit integer to the specified offset as little endian",
            constant: false,
        },
        Prototype {
            builtin: Builtin::WriteAddress,
            namespace: None,
            method: Some(Type::DynamicBytes),
            name: "writeAddress",
            params: vec![Type::Address(false), Type::Uint(32)],
            ret: vec![],
            target: vec![],
            doc: "Writes an address to the specified offset",
            constant: false,
        },
    ]
});

/// Does function call match builtin
pub fn is_builtin_call(namespace: Option<&str>, fname: &str, ns: &Namespace) -> bool {
    BUILTIN_FUNCTIONS.iter().any(|p| {
        p.name == fname
            && p.namespace == namespace
            && (p.target.is_empty() || p.target.contains(&ns.target))
    })
}

/// Get the prototype for a builtin. If the prototype has arguments, it is a function else
/// it is a variable.
pub fn get_prototype(builtin: Builtin) -> Option<&'static Prototype> {
    BUILTIN_FUNCTIONS
        .iter()
        .find(|p| p.builtin == builtin)
        .or_else(|| BUILTIN_VARIABLE.iter().find(|p| p.builtin == builtin))
        .or_else(|| BUILTIN_METHODS.iter().find(|p| p.builtin == builtin))
}

/// Does variable name match builtin
pub fn builtin_var(
    loc: &pt::Loc,
    namespace: Option<&str>,
    fname: &str,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Option<(Builtin, Type)> {
    if let Some(p) = BUILTIN_VARIABLE
        .iter()
        .find(|p| p.name == fname && p.namespace == namespace)
    {
        if p.target.is_empty() || p.target.contains(&ns.target) {
            if ns.target.is_substrate() && p.builtin == Builtin::Gasprice {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    String::from(
                        "use the function 'tx.gasprice(gas)' in stead, as 'tx.gasprice' may round down to zero. See https://solang.readthedocs.io/en/latest/language/builtins.html#gasprice",
                    ),
                ));
            }
            if ns.target == Target::Solana && p.builtin == Builtin::Value {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    String::from(
                        "Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer",
                    ),
                ));
            }
            return Some((p.builtin, p.ret[0].clone()));
        }
    }

    None
}

/// Does variable name match any builtin namespace
pub fn builtin_namespace(namespace: &str) -> bool {
    BUILTIN_VARIABLE
        .iter()
        .any(|p| p.namespace == Some(namespace))
}

/// Is name reserved for builtins
pub fn is_reserved(fname: &str) -> bool {
    if fname == "type" || fname == "super" {
        return true;
    }

    let is_builtin_function = BUILTIN_FUNCTIONS.iter().any(|p| {
        (p.name == fname && p.namespace.is_none() && p.method.is_none())
            || (p.namespace == Some(fname))
    });

    if is_builtin_function {
        return true;
    }

    BUILTIN_VARIABLE.iter().any(|p| {
        (p.name == fname && p.namespace.is_none() && p.method.is_none())
            || (p.namespace == Some(fname))
    })
}

/// Resolve a builtin call
pub fn resolve_call(
    loc: &pt::Loc,
    namespace: Option<&str>,
    id: &str,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let funcs = BUILTIN_FUNCTIONS
        .iter()
        .filter(|p| p.name == id && p.namespace == namespace && p.method.is_none())
        .collect::<Vec<&Prototype>>();
    let mut errors: Diagnostics = Diagnostics::default();

    for func in &funcs {
        let mut matches = true;

        if context.constant && !func.constant {
            errors.push(Diagnostic::cast_error(
                *loc,
                format!(
                    "cannot call function '{}' in constant expression",
                    func.name
                ),
            ));
            matches = false;
        }

        if func.params.len() != args.len() {
            errors.push(Diagnostic::cast_error(
                *loc,
                format!(
                    "builtin function '{}' expects {} arguments, {} provided",
                    func.name,
                    func.params.len(),
                    args.len()
                ),
            ));
            matches = false;
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let ty = func.params.get(i);

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                &mut errors,
                ty.map(ResolveTo::Type).unwrap_or(ResolveTo::Unknown),
            ) {
                Ok(e) => e,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            if let Some(ty) = ty {
                match arg.cast(&arg.loc(), ty, true, ns, &mut errors) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                    }
                }
            }
        }

        if !matches {
            if funcs.len() > 1 && diagnostics.extend_non_casting(&errors) {
                return Err(());
            }
        } else {
            // tx.gasprice(1) is a bad idea, just like tx.gasprice. Warn about this
            if ns.target.is_substrate() && func.builtin == Builtin::Gasprice {
                if let Ok((_, val)) = eval_const_number(&cast_args[0], ns) {
                    if val == BigInt::one() {
                        diagnostics.push(Diagnostic::warning(
                            *loc,
                            String::from(
                                "the function call 'tx.gasprice(1)' may round down to zero. See https://solang.readthedocs.io/en/latest/language/builtins.html#gasprice",
                            ),
                        ));
                    }
                }
            }

            return Ok(Expression::Builtin(
                *loc,
                func.ret.to_vec(),
                func.builtin,
                cast_args,
            ));
        }
    }

    if funcs.len() != 1 {
        diagnostics.push(Diagnostic::error(
            *loc,
            "cannot find overloaded function which matches signature".to_string(),
        ));
    } else {
        diagnostics.extend(errors);
    }

    Err(())
}

/// Resolve a builtin namespace call. The takes the unresolved arguments, since it has
/// to handle the special case "abi.decode(foo, (int32, bool, address))" where the
/// second argument is a type list. The generic expression resolver cannot deal with
/// this. It is only used in for this specific call.
pub fn resolve_namespace_call(
    loc: &pt::Loc,
    namespace: &str,
    name: &str,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    // The abi.* functions need special handling, others do not
    if namespace != "abi" {
        return resolve_call(
            loc,
            Some(namespace),
            name,
            args,
            context,
            ns,
            symtable,
            diagnostics,
        );
    }

    let builtin = match name {
        "decode" => Builtin::AbiDecode,
        "encode" => Builtin::AbiEncode,
        "encodePacked" => Builtin::AbiEncodePacked,
        "encodeWithSelector" => Builtin::AbiEncodeWithSelector,
        "encodeWithSignature" => Builtin::AbiEncodeWithSignature,
        "encodeCall" => Builtin::AbiEncodeCall,
        _ => unreachable!(),
    };

    if builtin == Builtin::AbiDecode {
        if args.len() != 2 {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("function expects {} arguments, {} provided", 2, args.len()),
            ));

            return Err(());
        }

        // first args
        let data = expression(
            &args[0],
            context,
            ns,
            symtable,
            diagnostics,
            ResolveTo::Type(&Type::DynamicBytes),
        )?
        .cast(&args[0].loc(), &Type::DynamicBytes, true, ns, diagnostics)?;

        let mut tys = Vec::new();
        let mut broken = false;

        match &args[1] {
            pt::Expression::List(_, list) => {
                for (loc, param) in list {
                    if let Some(param) = param {
                        let ty = ns.resolve_type(
                            context.file_no,
                            context.contract_no,
                            false,
                            &param.ty,
                            diagnostics,
                        )?;

                        if let Some(storage) = &param.storage {
                            diagnostics.push(Diagnostic::error(
                                storage.loc(),
                                format!("storage modifier '{}' not allowed", storage),
                            ));
                            broken = true;
                        }

                        if let Some(name) = &param.name {
                            diagnostics.push(Diagnostic::error(
                                name.loc,
                                format!("unexpected identifier '{}' in type", name.name),
                            ));
                            broken = true;
                        }

                        if ty.is_mapping() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "mapping cannot be abi decoded or encoded".to_string(),
                            ));
                            broken = true;
                        }

                        tys.push(ty);
                    } else {
                        diagnostics.push(Diagnostic::error(*loc, "missing type".to_string()));

                        broken = true;
                    }
                }
            }
            _ => {
                let ty = ns.resolve_type(
                    context.file_no,
                    context.contract_no,
                    false,
                    args[1].remove_parenthesis(),
                    diagnostics,
                )?;

                if ty.is_mapping() {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "mapping cannot be abi decoded or encoded".to_string(),
                    ));
                    broken = true;
                }

                tys.push(ty);
            }
        }

        return if broken {
            Err(())
        } else {
            Ok(Expression::Builtin(
                *loc,
                tys,
                Builtin::AbiDecode,
                vec![data],
            ))
        };
    }

    let mut resolved_args = Vec::new();
    let mut args_iter = args.iter();

    match builtin {
        Builtin::AbiEncodeWithSelector => {
            // first argument is selector
            if let Some(selector) = args_iter.next() {
                let selector = expression(
                    selector,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&Type::Bytes(4)),
                )?;

                resolved_args.insert(
                    0,
                    selector.cast(&selector.loc(), &Type::Bytes(4), true, ns, diagnostics)?,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "function requires one 'bytes4' selector argument".to_string(),
                ));

                return Err(());
            }
        }
        Builtin::AbiEncodeCall => {
            // first argument is function
            if let Some(function) = args_iter.next() {
                let function = expression(
                    function,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Unknown,
                )?;

                match function.ty() {
                    Type::ExternalFunction { params, .. }
                    | Type::InternalFunction { params, .. } => {
                        resolved_args.push(function);

                        if args.len() - 1 != params.len() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                format!(
                                    "function takes {} arguments, {} provided",
                                    params.len(),
                                    args.len() - 1
                                ),
                            ));

                            return Err(());
                        }

                        for (arg_no, arg) in args_iter.enumerate() {
                            let mut expr = expression(
                                arg,
                                context,
                                ns,
                                symtable,
                                diagnostics,
                                ResolveTo::Type(&params[arg_no]),
                            )?;

                            expr = expr.cast(&arg.loc(), &params[arg_no], true, ns, diagnostics)?;

                            // A string or hex literal should be encoded as a string
                            if let Expression::BytesLiteral(..) = &expr {
                                expr =
                                    expr.cast(&arg.loc(), &Type::String, true, ns, diagnostics)?;
                            }

                            resolved_args.push(expr);
                        }

                        return Ok(Expression::Builtin(
                            *loc,
                            vec![Type::DynamicBytes],
                            builtin,
                            resolved_args,
                        ));
                    }
                    ty => {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "first argument should be function, got '{}'",
                                ty.to_string(ns)
                            ),
                        ));

                        return Err(());
                    }
                }
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "least one function argument required".to_string(),
                ));

                return Err(());
            }
        }
        Builtin::AbiEncodeWithSignature => {
            // first argument is signature
            if let Some(signature) = args_iter.next() {
                let signature = expression(
                    signature,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&Type::String),
                )?;

                resolved_args.insert(
                    0,
                    signature.cast(&signature.loc(), &Type::String, true, ns, diagnostics)?,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "function requires one 'string' signature argument".to_string(),
                ));

                return Err(());
            }
        }
        _ => (),
    }

    for arg in args_iter {
        let mut expr = expression(arg, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
        let ty = expr.ty();

        if ty.is_mapping() {
            diagnostics.push(Diagnostic::error(
                arg.loc(),
                "mapping type not permitted".to_string(),
            ));

            return Err(());
        }

        expr = expr.cast(&arg.loc(), ty.deref_any(), true, ns, diagnostics)?;

        // A string or hex literal should be encoded as a string
        if let Expression::BytesLiteral(..) = &expr {
            expr = expr.cast(&arg.loc(), &Type::String, true, ns, diagnostics)?;
        }

        resolved_args.push(expr);
    }

    Ok(Expression::Builtin(
        *loc,
        vec![Type::DynamicBytes],
        builtin,
        resolved_args,
    ))
}

/// Resolve a builtin call
pub fn resolve_method_call(
    expr: &Expression,
    id: &pt::Identifier,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Expression>, ()> {
    let expr_ty = expr.ty();
    let funcs: Vec<_> = BUILTIN_METHODS
        .iter()
        .filter(|func| func.name == id.name && func.method.as_ref() == Some(&expr_ty))
        .collect();
    let mut errors = Diagnostics::default();

    for func in &funcs {
        let mut matches = true;

        if context.constant && !func.constant {
            diagnostics.push(Diagnostic::cast_error(
                id.loc,
                format!(
                    "cannot call function '{}' in constant expression",
                    func.name
                ),
            ));
            matches = false;
        }

        if func.params.len() != args.len() {
            errors.push(Diagnostic::cast_error(
                id.loc,
                format!(
                    "builtin function '{}' expects {} arguments, {} provided",
                    func.name,
                    func.params.len(),
                    args.len()
                ),
            ));
            matches = false;
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            // we may have arguments that parameters
            let ty = func.params.get(i);

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                &mut errors,
                ty.map(ResolveTo::Type).unwrap_or(ResolveTo::Unknown),
            ) {
                Ok(e) => e,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            if let Some(ty) = ty {
                match arg.cast(&arg.loc(), ty, true, ns, &mut errors) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        continue;
                    }
                }
            }
        }

        if !matches {
            if funcs.len() > 1 && diagnostics.extend_non_casting(&errors) {
                return Err(());
            }
        } else {
            cast_args.insert(0, expr.clone());

            let returns = if func.ret.is_empty() {
                vec![Type::Void]
            } else {
                func.ret.to_vec()
            };

            return Ok(Some(Expression::Builtin(
                id.loc,
                returns,
                func.builtin,
                cast_args,
            )));
        }
    }

    match funcs.len() {
        0 => Ok(None),
        1 => {
            diagnostics.extend(errors);

            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                id.loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));

            Err(())
        }
    }
}

impl Namespace {
    pub fn add_solana_builtins(&mut self) {
        let file_no = self.files.len();

        self.files.push(File {
            path: PathBuf::from("solana"),
            line_starts: Vec::new(),
            cache_no: None,
        });

        let id = pt::Identifier {
            loc: pt::Loc::Builtin,
            name: String::from("AccountInfo"),
        };

        assert!(self.add_symbol(
            file_no,
            None,
            &id,
            Symbol::Struct(pt::Loc::Builtin, StructType::AccountInfo)
        ));

        let id = pt::Identifier {
            loc: pt::Loc::Builtin,
            name: String::from("AccountMeta"),
        };

        assert!(self.add_symbol(
            file_no,
            None,
            &id,
            Symbol::Struct(pt::Loc::Builtin, StructType::AccountMeta)
        ));

        let mut func = Function::new(
            pt::Loc::Builtin,
            "create_program_address".to_string(),
            None,
            Vec::new(),
            pt::FunctionTy::Function,
            None,
            pt::Visibility::Public(None),
            vec![
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Array(
                        Box::new(Type::Slice(Box::new(Type::Bytes(1)))),
                        vec![ArrayLength::AnyFixed],
                    ),
                    ty_loc: None,
                    readonly: false,
                    indexed: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Address(false),
                    ty_loc: None,
                    readonly: false,
                    indexed: false,
                    recursive: false,
                },
            ],
            vec![Parameter {
                loc: pt::Loc::Builtin,
                id: None,
                ty: Type::Address(false),
                ty_loc: None,
                readonly: false,
                indexed: false,
                recursive: false,
            }],
            self,
        );

        func.has_body = true;

        let func_no = self.functions.len();
        let id = Identifier {
            name: func.name.to_owned(),
            loc: pt::Loc::Builtin,
        };

        self.functions.push(func);

        assert!(self.add_symbol(
            file_no,
            None,
            &id,
            Symbol::Function(vec![(pt::Loc::Builtin, func_no)])
        ));

        let mut func = Function::new(
            pt::Loc::Builtin,
            "try_find_program_address".to_string(),
            None,
            Vec::new(),
            pt::FunctionTy::Function,
            None,
            pt::Visibility::Public(None),
            vec![
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Array(
                        Box::new(Type::Slice(Box::new(Type::Bytes(1)))),
                        vec![ArrayLength::AnyFixed],
                    ),
                    ty_loc: None,
                    readonly: false,
                    indexed: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Address(false),
                    ty_loc: None,
                    readonly: false,
                    indexed: false,
                    recursive: false,
                },
            ],
            vec![
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Address(false),
                    ty_loc: None,
                    readonly: false,
                    indexed: false,
                    recursive: false,
                },
                Parameter {
                    loc: pt::Loc::Builtin,
                    id: None,
                    ty: Type::Bytes(1),
                    ty_loc: None,
                    readonly: false,
                    indexed: false,
                    recursive: false,
                },
            ],
            self,
        );

        func.has_body = true;

        let func_no = self.functions.len();
        let id = Identifier {
            name: func.name.to_owned(),
            loc: pt::Loc::Builtin,
        };

        self.functions.push(func);

        assert!(self.add_symbol(
            file_no,
            None,
            &id,
            Symbol::Function(vec![(pt::Loc::Builtin, func_no)])
        ));
    }
}
