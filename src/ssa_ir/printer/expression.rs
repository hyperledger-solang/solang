// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::StringLocation;
use crate::ssa_ir::expressions::{Expr, Operand};
use crate::ssa_ir::printer::Printer;
use std::io::Write;

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
            Operand::BoolLiteral { value, .. } => write!(f, "{}", value),
            Operand::NumberLiteral { value, ty, .. } => write!(f, "{}({})", ty, value),
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
                self.print_rhs_operand(f, left)?;
                write!(f, " {} ", op)?;
                self.print_rhs_operand(f, right)
            }
            Expr::UnaryExpr {
                operator: op,
                right,
                ..
            } => {
                write!(f, "{}", op)?;
                self.print_rhs_operand(f, right)
            }
            Expr::Id { id, .. } => {
                let ty = self.get_var_type(id).unwrap();
                let name = self.get_var_name(id).unwrap();
                write!(f, "{}(%{})", ty, name)
            }
            Expr::ArrayLiteral { ty, values, .. } => {
                write!(f, "{}", ty)?;
                write!(f, " [")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, val).unwrap();
                });
                write!(f, "]")
            }
            Expr::ConstArrayLiteral { ty, values, .. } => {
                write!(f, "const {}", ty)?;
                write!(f, " [")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, val).unwrap();
                });
                write!(f, "]")
            }
            Expr::BytesLiteral { ty, value, .. } => {
                write!(f, "{} hex\"", ty)?;
                value.iter().enumerate().for_each(|(i, byte)| {
                    if i != 0 {
                        write!(f, "_").unwrap();
                    }
                    write!(f, "{:02x}", byte).unwrap();
                });
                write!(f, "\"")
            }
            Expr::StructLiteral { values, .. } => {
                write!(f, "struct {{ ")?;
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, val).unwrap();
                });
                write!(f, " }}")
            }
            Expr::Cast {
                operand: op, to_ty, ..
            } => {
                write!(f, "(cast ")?;
                self.print_rhs_operand(f, op)?;
                write!(f, " to {})", to_ty)
            }
            Expr::BytesCast { operand, to_ty, .. } => {
                write!(f, "(cast ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::SignExt { to_ty, operand, .. } => {
                write!(f, "(sext ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::ZeroExt { to_ty, operand, .. } => {
                write!(f, "(zext ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::Trunc { operand, to_ty, .. } => {
                write!(f, "(trunc ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " to {})", to_ty)
            }
            Expr::AllocDynamicBytes {
                ty,
                size,
                initializer,
                ..
            } => {
                if initializer.is_none() {
                    write!(f, "alloc {}[", ty)?;
                    self.print_rhs_operand(f, size)?;
                    return write!(f, "]");
                }

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
                write!(f, "&")?;
                self.print_rhs_operand(f, operand)
            }
            Expr::Load { operand, .. } => {
                write!(f, "*")?;
                self.print_rhs_operand(f, operand)
            }
            Expr::StructMember {
                operand, member, ..
            } => {
                write!(f, "access ")?;
                self.print_rhs_operand(f, operand)?;
                write!(f, " member {}", member)
            }
            Expr::Subscript { arr, index, .. } => {
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
                write!(f, "ptr_add(")?;
                self.print_rhs_operand(f, pointer)?;
                write!(f, ", ")?;
                self.print_rhs_operand(f, bytes_offset)?;
                write!(f, ")")
            }
            Expr::FunctionArg { arg_no, ty, .. } => {
                write!(f, "{}(arg#{})", ty, arg_no)
            }
            Expr::FormatString { args, .. } => {
                write!(f, "fmt_str(")?;
                args.iter().enumerate().for_each(|(i, (spec, arg))| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    let spec_str = spec.to_string();
                    if spec_str.is_empty() {
                        self.print_rhs_operand(f, arg).unwrap();
                    } else {
                        write!(f, "{} ", spec).unwrap();
                        self.print_rhs_operand(f, arg).unwrap();
                    }
                });
                write!(f, ")")
            }
            Expr::InternalFunctionCfg { cfg_no, .. } => write!(f, "function#{}", cfg_no),
            Expr::Keccak256 { args, .. } => {
                write!(f, "keccak256(")?;
                args.iter().enumerate().for_each(|(i, arg)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, arg).unwrap();
                });
                write!(f, ")")
            }
            Expr::StringCompare { left, right, .. } => {
                write!(f, "strcmp(")?;
                match left {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s)?,
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op)?,
                };

                write!(f, ", ")?;

                match right {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s)?,
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op)?,
                };
                write!(f, ")")
            }
            Expr::StringConcat { left, right, .. } => {
                write!(f, "strcat(")?;
                match left {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s)?,
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op)?,
                };

                write!(f, ", ")?;

                match right {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s)?,
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op)?,
                };
                write!(f, ")")
            }
            Expr::StorageArrayLength { array, .. } => {
                write!(f, "storage_arr_len(")?;
                self.print_rhs_operand(f, array)?;
                write!(f, ")")
            }
            Expr::ReturnData { .. } => write!(f, "(extern_call_ret_data)"),
            Expr::NumberLiteral { value, .. } => {
                write!(f, "{}", value)
            }
            Expr::BoolLiteral { value, .. } => write!(f, "{}", value),
            Expr::Builtin { kind, args, .. } => {
                write!(f, "builtin: {:?}(", kind)?;
                args.iter().enumerate().for_each(|(i, arg)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, arg).unwrap();
                });
                write!(f, ")")
            }
        }
    }
}
