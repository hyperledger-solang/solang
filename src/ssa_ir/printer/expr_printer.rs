use crate::sema::ast::StringLocation;
use crate::ssa_ir::expr::{Expr, Operand};
use crate::ssa_ir::printer::Printer;
use crate::ssa_ir::ssa_type::Type;
use std::io::Write;

#[macro_export]
macro_rules! stringfy_expr {
    ($printer:expr, $expr:expr) => {{
        let mut buffer = Vec::new();
        $printer.print_expr(&mut buffer, $expr).unwrap(); // you may want to handle this unwrap in a different way
        String::from_utf8(buffer).expect("Failed to convert to string")
    }};
}

#[macro_export]
macro_rules! stringfy_rhs_operand {
    ($printer:expr, $operand:expr) => {{
        let mut buffer = Vec::new();
        $printer.print_rhs_operand(&mut buffer, $operand).unwrap();
        String::from_utf8(buffer).expect("Failed to convert to string")
    }};
}

#[macro_export]
macro_rules! stringfy_lhs_operand {
    ($printer:expr, $operand:expr) => {{
        let mut buffer = Vec::new();
        $printer.print_lhs_operand(&mut buffer, $operand).unwrap();
        String::from_utf8(buffer).expect("Failed to convert to string")
    }};
}

impl Printer {
    pub fn print_lhs_operand(&self, f: &mut dyn Write, operand: &Operand) -> std::io::Result<()> {
        match operand {
            Operand::Id { id, .. } => {
                let ty = self.get_var_type(id).unwrap();
                let name = self.get_var_name(id).unwrap();
                write!(f, "{} %{}", ty, name)
            }
            _ => panic!("unsupported lhs operand: {:?}", operand),
        }
    }

    pub fn print_rhs_operand(&self, f: &mut dyn Write, operand: &Operand) -> std::io::Result<()> {
        match operand {
            Operand::Id { id, .. } => {
                let ty = self.get_var_type(id).unwrap();
                let name = self.get_var_name(id).unwrap();
                write!(f, "{}(%{})", ty, name)
            }
            Operand::BoolLiteral { value } => write!(f, "{}", value),
            Operand::NumberLiteral { value, ty } => write!(f, "{}({})", ty, value),
        }
    }

    pub fn print_expr(&self, f: &mut dyn Write, expr: &Expr) -> std::io::Result<()> {
        match expr {
            Expr::BinaryExpr {
                operator: op,
                left,
                right,
                ..
            } => {
                // write!(f, "{} {} {}", left, op, right)
                self.print_rhs_operand(f, left)?;
                write!(f, " {} ", op)?;
                self.print_rhs_operand(f, right)
            }
            Expr::UnaryExpr {
                operator: op,
                right,
                ..
            } => {
                // write!(f, "{}{}", op, right)
                write!(f, "{}", op)?;
                self.print_rhs_operand(f, right)
            }
            Expr::Id { id, .. } => {
                // write!(f, "%{}", id)
                let ty = self.get_var_type(id).unwrap();
                let name = self.get_var_name(id).unwrap();
                write!(f, "{}(%{})", ty, name)
            }
            Expr::ArrayLiteral { ty, values, .. } => {
                // for array ty: uint8, dimensions: [2][2], values [1, 2, %3], we want to print
                // uint8[2][2] [1, 2, %3]
                write!(f, "{}", ty)?;
                write!(f, " [")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    // write!(f, "{}", val).unwrap();
                    self.print_rhs_operand(f, val).unwrap();
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
                    // write!(f, "{}", val).unwrap();
                    self.print_rhs_operand(f, val).unwrap();
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
                    // write!(f, "{}", val).unwrap();
                    self.print_rhs_operand(f, val).unwrap();
                });
                write!(f, " }}")
            }
            Expr::Cast {
                operand: op, to_ty, ..
            } => {
                // example: cast %1 to uint8 can be written as: (%1 as uint8)
                // write!(f, "(cast {} as {})", op, to_ty)
                write!(f, "(cast ")?;
                self.print_rhs_operand(f, op)?;
                write!(f, " to {})", to_ty)
            }
            Expr::BytesCast { operand, to_ty, .. } => {
                // example: casting from a dynamic bytes to a fixed bytes:
                //          %1 of ptr<bytes2> to bytes4
                //          can be written as: (bytes* %1 as bytes4)

                // example: casting from a fixed bytes to a dynamic bytes:
                //          %1 of bytes4 to ptr<bytes8>
                //          can be written as: (bytes4 %1 as bytes8*)
                // write!(f, "(cast {} as {})", operand, to_ty)
                write!(f, "(cast ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::SignExt { to_ty, operand, .. } => {
                // example: sign extending a int8 to int16:
                //          %1 of int8 to int16
                //          can be written as: (sext %1 to int16)
                // write!(f, "(sext {} to {})", operand, to_ty)
                write!(f, "(sext ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::ZeroExt { to_ty, operand, .. } => {
                // example: zero extending a uint8 to uint16:
                //          %1 of uint8 to uint16
                //          can be written as: (zext %1 to uint16)
                // write!(f, "(zext {} to {})", operand, to_ty)
                write!(f, "(zext ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::Trunc { operand, to_ty, .. } => {
                // example: truncating a uint16 to uint8:
                //          %1 of uint16 to uint8
                //          can be written as: (trunc %1 to uint8)
                // write!(f, "(trunc {} to {})", operand, to_ty)
                write!(f, "(trunc ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
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
                    // return write!(f, "alloc {}[{}]", ty, size);
                    write!(f, "alloc {}[", ty)?;
                    self.print_rhs_operand(f, size)?;
                    return write!(f, "]");
                }

                // case2: allocating a dynamic bytes with initializer:
                //        Solidity: bytes memory a = new bytes(3) { 0x01, 0x02, 0x03 };
                //        rhs print: alloc bytes1[uint8(3)] {0x01, 0x02, 0x03}
                // write!(f, "alloc {}[{}] {{", ty, size)?;
                write!(f, "alloc {}[", ty)?;
                self.print_rhs_operand(f, size)?;
                write!(f, "] {{")?;
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
                // write!(f, "&{}", operand)
                write!(f, "&")?;
                self.print_rhs_operand(f, operand)
            }
            Expr::Load { operand, .. } => {
                // this is the value-of-address operator
                // example: *%1
                // write!(f, "*{}", operand)
                write!(f, "*")?;
                self.print_rhs_operand(f, operand)
            }
            // example: uint8 %1->1
            Expr::StructMember {
                operand, member, ..
            } => {
                // write!(f, "{}->{}", operand, member)
                self.print_rhs_operand(f, operand)?;
                write!(f, "->{}", member)
            }
            Expr::Subscript { arr, index, .. } => {
                // example: ptr<uint8[2]> %1[uint8(0)]
                // write!(f, "{}[{}]", arr, index)
                self.print_rhs_operand(f, arr)?;
                write!(f, "[")?;
                self.print_rhs_operand(f, index)?;
                write!(f, "]")
            }
            Expr::AdvancePointer {
                pointer,
                bytes_offset,
                ..
            } => {
                // example: ptr_add(%1, %2)
                // write!(f, "ptr_add({}, {})", pointer, bytes_offset)
                write!(f, "ptr_add(")?;
                self.print_rhs_operand(f, pointer)?;
                write!(f, ", ")?;
                self.print_rhs_operand(f, bytes_offset)?;
                write!(f, ")")
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
                        // write!(f, "{}", arg).unwrap();
                        self.print_rhs_operand(f, arg).unwrap();
                    } else {
                        // write!(f, "{} {}", spec, arg).unwrap();
                        write!(f, "{} ", spec).unwrap();
                        self.print_rhs_operand(f, arg).unwrap();
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
                    // write!(f, "{}", arg).unwrap();
                    self.print_rhs_operand(f, arg).unwrap();
                });
                write!(f, ")")
            }
            Expr::StringCompare { left, right, .. } => {
                // case1: strcmp(%1, %2)
                // case2: strcmp("[97, 98, 99]", %1)
                // case3: strcmp(%1, "[97, 98, 99]")
                let left_str = match left {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => {
                        stringfy_rhs_operand!(self, op)
                    }
                };
                let right_str = match right {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => stringfy_rhs_operand!(self, op),
                };
                write!(f, "strcmp({}, {})", left_str, right_str)
            }
            Expr::StringConcat { left, right, .. } => {
                // case1: strcat(%1, %2)
                // case2: strcat("[97, 98, 99]", %1)
                // case3: strcat(%1, "[97, 98, 99]")
                let left_str = match left {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => stringfy_rhs_operand!(self, op),
                };
                let right_str = match right {
                    StringLocation::CompileTime(s) => format!("\"{:?}\"", s),
                    StringLocation::RunTime(op) => stringfy_rhs_operand!(self, op),
                };
                write!(f, "strcat({}, {})", left_str, right_str)
            }
            Expr::StorageArrayLength { array, .. } => {
                // example: storage_arr_len(uint8[] %1)
                // write!(f, "storage_arr_len({})", array)
                write!(f, "storage_arr_len(")?;
                self.print_rhs_operand(f, array)?;
                write!(f, ")")
            }
            Expr::ReturnData { .. } => write!(f, "(extern_call_ret_data)"),
            Expr::NumberLiteral { value, .. } => {
                // example: 3
                write!(f, "{}", value)
            }
            Expr::BoolLiteral { value, .. } => write!(f, "{}", value),
            _ => panic!("unsupported expr: {:?}", expr),
        }
    }
}
