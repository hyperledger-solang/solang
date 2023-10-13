// SPDX-License-Identifier: Apache-2.0
use crate::sema::ast::{FormatArg, StringLocation};
use crate::ssa_ir::ssa_type::Type;
use num_bigint::BigInt;
use solang_parser::pt::Loc;
use std::fmt;
use std::fmt::Formatter;

/// Three-address code type, which is a subset of the Solidity AST
// FIXME Be careful about the data types: pointers, primitives, and references.

/// Three-address code identifier
/// Variable and Literal
#[derive(Clone, Debug)]
pub enum Operand {
    Id { id: usize },
    BoolLiteral { value: bool },
    NumberLiteral { value: BigInt, ty: Type },
}

/// binary operators
// LLVM doesn't diff signed and unsigned
#[derive(Debug)]
pub enum BinaryOperator {
    Add { overflowing: bool },
    Sub { overflowing: bool },
    Mul { overflowing: bool },
    Pow { overflowing: bool },

    Div,
    UDiv,

    Mod,
    UMod,

    Eq,
    Neq,

    Lt,
    ULt,

    Lte,
    ULte,

    Gt,
    UGt,

    Gte,
    UGte,

    BitAnd,
    BitOr,
    BitXor,

    Shl,
    Shr,
    UShr,
}

#[derive(Debug)]
/// unary operators
pub enum UnaryOperator {
    Not,
    Neg { overflowing: bool },
    BitNot,
}

#[derive(Debug)]
pub enum Expr {
    BinaryExpr {
        loc: Loc,
        operator: BinaryOperator,
        left: Box<Operand>,
        right: Box<Operand>,
    },
    UnaryExpr {
        loc: Loc,
        operator: UnaryOperator,
        right: Box<Operand>,
    },

    Id {
        loc: Loc,
        id: usize,
    },

    /*************************** Constants ***************************/
    ArrayLiteral {
        loc: Loc,
        // Dynamic type in array literal is impossible
        ty: Type,
        dimensions: Vec<u32>,
        values: Vec<Operand>,
    },
    ConstArrayLiteral {
        loc: Loc,
        ty: Type,
        dimensions: Vec<u32>,
        values: Vec<Operand>,
    },
    BytesLiteral {
        loc: Loc,
        ty: Type,
        value: Vec<u8>,
    },
    StructLiteral {
        loc: Loc,
        ty: Type,
        values: Vec<Operand>,
    },

    /*************************** Casts ***************************/
    Cast {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    BytesCast {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    // Used for signed integers: int8 -> int16
    // https://en.wikipedia.org/wiki/Sign_extension
    SignExt {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    // extending the length, only for unsigned int
    ZeroExt {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },
    // truncating integer into a shorter one
    Trunc {
        loc: Loc,
        operand: Box<Operand>,
        to_ty: Type,
    },

    /*************************** Memory Alloc ***************************/
    AllocDynamicBytes {
        loc: Loc,
        ty: Type,
        size: Box<Operand>,
        initializer: Option<Vec<u8>>,
    },

    /*************************** Memory Access ***************************/
    // address-of
    GetRef {
        loc: Loc,
        operand: Box<Operand>,
    },
    // value-of-address
    Load {
        loc: Loc,
        operand: Box<Operand>,
    },
    // Used for accessing struct member
    StructMember {
        loc: Loc,
        operand: Box<Operand>,
        member: usize,
    },
    // Array subscripting: <array>[<index>]
    Subscript {
        loc: Loc,
        arr: Box<Operand>,
        index: Box<Operand>,
    },
    // [b1, b2, b3]
    AdvancePointer {
        pointer: Box<Operand>,
        bytes_offset: Box<Operand>,
    },
    // get the nth param in the current function call stack
    FunctionArg {
        loc: Loc,
        arg_no: usize,
    },

    /*************************** Function Calls ***************************/
    FormatString {
        loc: Loc,
        args: Vec<(FormatArg, Operand)>,
    },
    InternalFunctionCfg {
        cfg_no: usize,
    },
    // hash function
    Keccak256 {
        loc: Loc,
        args: Vec<Operand>,
    },
    StringCompare {
        loc: Loc,
        left: StringLocation<Operand>,
        right: StringLocation<Operand>,
    },
    StringConcat {
        loc: Loc,
        left: StringLocation<Operand>,
        right: StringLocation<Operand>,
    },

    /*************************** RPC Calls ***************************/
    // a storage array is in the account
    // this func is a len() function
    StorageArrayLength {
        loc: Loc,
        array: Box<Operand>,
    },
    // External call: represents a hard coded mem location
    ReturnData {
        loc: Loc,
    },
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Id { id, .. } => write!(f, "%{}", id),
            Operand::BoolLiteral { value } => write!(f, "{}", value),
            Operand::NumberLiteral { value, ty } => write!(f, "{}({})", ty, value),
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOperator::Add { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)+" } else { "+" })
            }
            BinaryOperator::Sub { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)-" } else { "-" })
            }
            BinaryOperator::Mul { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)*" } else { "*" })
            }
            BinaryOperator::Pow { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)**" } else { "**" })
            }
            BinaryOperator::Div => write!(f, "/"),
            // example: uint8 a = b (u)/ c
            BinaryOperator::UDiv => write!(f, "(u)/"),
            BinaryOperator::Mod => write!(f, "%"),
            BinaryOperator::UMod => write!(f, "(u)%"),
            BinaryOperator::Eq => write!(f, "=="),
            BinaryOperator::Neq => write!(f, "!="),
            BinaryOperator::Lt => write!(f, "<"),
            BinaryOperator::ULt => write!(f, "(u)<"),
            BinaryOperator::Lte => write!(f, "<="),
            BinaryOperator::ULte => write!(f, "(u)<="),
            BinaryOperator::Gt => write!(f, ">"),
            BinaryOperator::UGt => write!(f, "(u)>"),
            BinaryOperator::Gte => write!(f, ">="),
            BinaryOperator::UGte => write!(f, "(u)>="),
            BinaryOperator::BitAnd => write!(f, "&"),
            BinaryOperator::BitOr => write!(f, "|"),
            BinaryOperator::BitXor => write!(f, "^"),
            BinaryOperator::Shl => write!(f, "<<"),
            BinaryOperator::Shr => write!(f, ">>"),
            BinaryOperator::UShr => write!(f, "(u)>>"),
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOperator::Not => write!(f, "!"),
            UnaryOperator::Neg { overflowing } => {
                write!(f, "{}", if *overflowing { "(of)-" } else { "-" })
            }
            UnaryOperator::BitNot => write!(f, "~"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Expr::BinaryExpr {
                operator: op,
                left,
                right,
                ..
            } => write!(f, "{} {} {}", left, op, right),
            Expr::UnaryExpr {
                operator: op,
                right,
                ..
            } => write!(f, "{}{}", op, right),
            Expr::Id { id, .. } => write!(f, "%{}", id),
            Expr::ArrayLiteral { ty, values, .. } => {
                // for array ty: uint8, dimensions: [2][2], values [1, 2, %3], we want to print
                // uint8[2][2] [1, 2, %3]
                write!(f, "{}", ty)?;
                write!(f, " [")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{}", val).unwrap();
                });
                write!(f, "]")
            }
            Expr::ConstArrayLiteral { ty, values, .. } => {
                // for array ty: uint8, dimensions: [2][2], values [1, 2, %3], we want to print
                // const uint8[2][2] [1, 2, %3]
                write!(f, "const {}", ty)?;
                write!(f, " [")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{}", val).unwrap();
                });
                write!(f, "]")
            }
            Expr::BytesLiteral { ty, value, .. } => {
                // example: bytes4 hex"41_42_43_44";
                write!(f, "{} hex\"", ty)?;
                // the bytes should be separated by _
                value.iter().enumerate().for_each(|(i, byte)| {
                    if i != 0 {
                        write!(f, "_").unwrap();
                    }
                    write!(f, "{:02x}", byte).unwrap();
                });
                write!(f, "\"")
            }
            Expr::StructLiteral { values, .. } => {
                // for any struct, we want to print: struct { <values> }
                // for example: struct { uint8(1), uint8(2) }
                write!(f, "struct {{ ")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{}", val).unwrap();
                });
                write!(f, " }}")
            }
            Expr::Cast {
                operand: op, to_ty, ..
            } => {
                // example: cast %1 to uint8 can be written as: (%1 as uint8)
                write!(f, "(cast {} as {})", op, to_ty)
            }
            Expr::BytesCast { operand, to_ty, .. } => {
                // example: casting from a dynamic bytes to a fixed bytes:
                //          %1 of ptr<bytes2> to bytes4
                //          can be written as: (bytes* %1 as bytes4)

                // example: casting from a fixed bytes to a dynamic bytes:
                //          %1 of bytes4 to ptr<bytes8>
                //          can be written as: (bytes4 %1 as bytes8*)
                write!(f, "(cast {} as {})", operand, to_ty)
            }
            Expr::SignExt { to_ty, operand, .. } => {
                // example: sign extending a int8 to int16:
                //          %1 of int8 to int16
                //          can be written as: (sext %1 to int16)
                write!(f, "(sext {} to {})", operand, to_ty)
            }
            Expr::ZeroExt { to_ty, operand, .. } => {
                // example: zero extending a uint8 to uint16:
                //          %1 of uint8 to uint16
                //          can be written as: (zext %1 to uint16)
                write!(f, "(zext {} to {})", operand, to_ty)
            }
            Expr::Trunc { operand, to_ty, .. } => {
                // example: truncating a uint16 to uint8:
                //          %1 of uint16 to uint8
                //          can be written as: (trunc %1 to uint8)
                write!(f, "(trunc {} to {})", operand, to_ty)
            }
            Expr::AllocDynamicBytes {
                ty: Type::Ptr(ty),
                size,
                initializer,
                ..
            } => {
                // case1: allocating a dynamic bytes without initializer:
                //        Solidity: bytes memory a = new bytes(10);
                //        rhs print: alloc bytes1[uint8(10)]
                if initializer.is_none() {
                    return write!(f, "alloc {}[{}]", ty, size);
                }

                // case2: allocating a dynamic bytes with initializer:
                //        Solidity: bytes memory a = new bytes(3) { 0x01, 0x02, 0x03 };
                //        rhs print: alloc bytes1[uint8(3)] {0x01, 0x02, 0x03}
                write!(f, "alloc {}[{}] {{", ty, size)?;
                initializer
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .for_each(|(i, byte)| {
                        if i != 0 {
                            write!(f, ", ").unwrap();
                        }
                        write!(f, "{:02x}", byte).unwrap();
                    });
                write!(f, "}}")
            }
            Expr::GetRef { operand, .. } => {
                // this is the address-of operator
                // example: &%1
                write!(f, "&{}", operand)
            }
            Expr::Load { operand, .. } => {
                // this is the value-of-address operator
                // example: *%1
                write!(f, "*{}", operand)
            }
            // example: uint8 %1->1
            Expr::StructMember {
                operand, member, ..
            } => write!(f, "{}->{}", operand, member),
            Expr::Subscript { arr, index, .. } => {
                // example: ptr<uint8[2]> %1[uint8(0)]
                write!(f, "{}[{}]", arr, index)
            }
            Expr::AdvancePointer {
                pointer,
                bytes_offset,
                ..
            } => {
                // example: ptr_add(%1, %2)
                write!(f, "ptr_add({}, {})", pointer, bytes_offset)
            }
            Expr::FunctionArg { arg_no, .. } => {
                // example: the 2nd arg of type uint8
                //          (uint8 arg#2)
                write!(f, "arg#{}", arg_no)
            }
            Expr::FormatString { args, .. } => {
                write!(f, "fmt_str(")?;
                args.iter().enumerate().for_each(|(i, (spec, arg))| {
                    // case1: spec is empty:
                    //        fmt_str(%1)
                    // case2: spec is binary:
                    //        fmt_str(:b %1)
                    // case3: spec is hex:
                    //        fmt_str(:x %1)
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    // spec_str will be either: "" or ":b", or ":x"
                    let spec_str = spec.to_string();
                    if spec_str.is_empty() {
                        write!(f, "{}", arg).unwrap();
                    } else {
                        write!(f, "{} {}", spec, arg).unwrap();
                    }
                });
                write!(f, ")")
            }
            Expr::InternalFunctionCfg { cfg_no, .. } => write!(f, "function#{}", cfg_no),
            Expr::Keccak256 { args, .. } => {
                // example: keccak256(%1, %2)
                write!(f, "keccak256(")?;
                args.iter().enumerate().for_each(|(i, arg)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    write!(f, "{}", arg).unwrap();
                });
                write!(f, ")")
            }
            Expr::StringCompare { left, right, .. } => {
                // case1: strcmp(%1, %2)
                // case2: strcmp("[97, 98, 99]", %1)
                // case3: strcmp(%1, "[97, 98, 99]")
                let left_str = match left {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => format!("{}", op),
                };
                let right_str = match right {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => format!("{}", op),
                };
                write!(f, "strcmp({}, {})", left_str, right_str)
            }
            Expr::StringConcat { left, right, .. } => {
                // case1: strcat(%1, %2)
                // case2: strcat("[97, 98, 99]", %1)
                // case3: strcat(%1, "[97, 98, 99]")
                let left_str = match left {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => format!("{}", op),
                };
                let right_str = match right {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => format!("{}", op),
                };
                write!(f, "strcat({}, {})", left_str, right_str)
            }
            Expr::StorageArrayLength { array, .. } => {
                // example: storage_arr_len(uint8[] %1)
                write!(f, "storage_arr_len({})", array)
            }
            Expr::ReturnData { .. } => write!(f, "(extern_call_ret_data)"),
            _ => panic!("unsupported expr: {:?}", self),
        }
    }
}
