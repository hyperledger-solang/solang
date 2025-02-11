// SPDX-License-Identifier: Apache-2.0

use crate::lir::expressions::{Expression, Operand};
use crate::lir::printer::Printer;
use crate::sema::ast::StringLocation;
use std::io::Write;

impl Printer<'_> {
    /// print left-hand-side operand
    pub fn print_lhs_operand(&self, f: &mut dyn Write, operand: &Operand) {
        match operand {
            Operand::Id { id, .. } => {
                let ty = self.get_var_type(id);
                let name = self.get_var_name(id);
                write!(f, "{} %{}", ty, name).unwrap();
            }
            _ => unreachable!("unsupported lhs operand: {:?}", operand),
        }
    }

    /// print right-hand-side operand
    pub fn print_rhs_operand(&self, f: &mut dyn Write, operand: &Operand) {
        match operand {
            Operand::Id { id, .. } => {
                let ty = self.get_var_type(id);
                let name = self.get_var_name(id);
                write!(f, "{}(%{})", ty, name).unwrap();
            }
            Operand::BoolLiteral { value, .. } => write!(f, "{}", value).unwrap(),
            Operand::NumberLiteral { value, ty, .. } => write!(f, "{}({})", ty, value).unwrap(),
        }
    }

    pub fn print_expr(&self, f: &mut dyn Write, expr: &Expression) {
        match expr {
            Expression::BinaryExpr {
                operator: op,
                left,
                right,
                ..
            } => {
                self.print_rhs_operand(f, left);
                write!(f, " {} ", op).unwrap();
                self.print_rhs_operand(f, right);
            }
            Expression::UnaryExpr {
                operator: op,
                right,
                ..
            } => {
                write!(f, "{}", op).unwrap();
                self.print_rhs_operand(f, right);
            }
            Expression::Id { id, .. } => {
                let ty = self.get_var_type(id);
                let name = self.get_var_name(id);
                write!(f, "{}(%{})", ty, name).unwrap()
            }
            Expression::ArrayLiteral { ty, values, .. } => {
                write!(f, "{}", ty).unwrap();
                write!(f, " [").unwrap();
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, val);
                });
                write!(f, "]").unwrap()
            }
            Expression::ConstArrayLiteral { ty, values, .. } => {
                write!(f, "const {}", ty).unwrap();
                write!(f, " [").unwrap();
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, val);
                });
                write!(f, "]").unwrap();
            }
            Expression::BytesLiteral { ty, value, .. } => {
                write!(f, "{} hex\"", ty).unwrap();
                value.iter().enumerate().for_each(|(i, byte)| {
                    if i != 0 {
                        write!(f, "_").unwrap();
                    }
                    write!(f, "{:02x}", byte).unwrap();
                });
                write!(f, "\"").unwrap();
            }
            Expression::StructLiteral { values, .. } => {
                write!(f, "struct {{ ").unwrap();
                values.iter().enumerate().for_each(|(i, val)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, val);
                });
                write!(f, " }}").unwrap();
            }
            Expression::Cast {
                operand: op, to_ty, ..
            } => {
                write!(f, "(cast ").unwrap();
                self.print_rhs_operand(f, op);
                write!(f, " to {})", to_ty).unwrap();
            }
            Expression::BytesCast { operand, to_ty, .. } => {
                write!(f, "(cast ").unwrap();
                self.print_rhs_operand(f, operand);
                write!(f, " to {})", to_ty).unwrap();
            }
            Expression::SignExt { to_ty, operand, .. } => {
                write!(f, "(sext ").unwrap();
                self.print_rhs_operand(f, operand);
                write!(f, " to {})", to_ty).unwrap();
            }
            Expression::ZeroExt { to_ty, operand, .. } => {
                write!(f, "(zext ").unwrap();
                self.print_rhs_operand(f, operand);
                write!(f, " to {})", to_ty).unwrap();
            }
            Expression::Trunc { operand, to_ty, .. } => {
                write!(f, "(trunc ").unwrap();
                self.print_rhs_operand(f, operand);
                write!(f, " to {})", to_ty).unwrap();
            }
            Expression::AllocDynamicBytes {
                ty,
                size,
                initializer,
                ..
            } => {
                if initializer.is_none() {
                    write!(f, "alloc {}[", ty).unwrap();
                    self.print_rhs_operand(f, size);
                    return write!(f, "]").unwrap();
                }

                write!(f, "alloc {}[", ty).unwrap();
                self.print_rhs_operand(f, size);
                write!(f, "] {{").unwrap();
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
                write!(f, "}}").unwrap();
            }
            Expression::GetRef { operand, .. } => {
                write!(f, "&").unwrap();
                self.print_rhs_operand(f, operand)
            }
            Expression::Load { operand, .. } => {
                write!(f, "*").unwrap();
                self.print_rhs_operand(f, operand)
            }
            Expression::StructMember {
                operand, member, ..
            } => {
                write!(f, "access ").unwrap();
                self.print_rhs_operand(f, operand);
                write!(f, " member {}", member).unwrap();
            }
            Expression::Subscript { arr, index, .. } => {
                self.print_rhs_operand(f, arr);
                write!(f, "[").unwrap();
                self.print_rhs_operand(f, index);
                write!(f, "]").unwrap();
            }
            Expression::AdvancePointer {
                pointer,
                bytes_offset,
                ..
            } => {
                write!(f, "ptr_add(").unwrap();
                self.print_rhs_operand(f, pointer);
                write!(f, ", ").unwrap();
                self.print_rhs_operand(f, bytes_offset);
                write!(f, ")").unwrap();
            }
            Expression::FunctionArg { arg_no, ty, .. } => {
                write!(f, "{}(arg#{})", ty, arg_no).unwrap();
            }
            Expression::FormatString { args, .. } => {
                write!(f, "fmt_str(").unwrap();
                args.iter().enumerate().for_each(|(i, (spec, arg))| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    let spec_str = spec.to_string();
                    if spec_str.is_empty() {
                        self.print_rhs_operand(f, arg);
                    } else {
                        write!(f, "{} ", spec).unwrap();
                        self.print_rhs_operand(f, arg);
                    }
                });
                write!(f, ")").unwrap();
            }
            Expression::InternalFunctionCfg { cfg_no, .. } => {
                write!(f, "function#{}", cfg_no).unwrap()
            }
            Expression::Keccak256 { args, .. } => {
                write!(f, "keccak256(").unwrap();
                args.iter().enumerate().for_each(|(i, arg)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, arg);
                });
                write!(f, ")").unwrap();
            }
            Expression::StringCompare { left, right, .. } => {
                write!(f, "strcmp(").unwrap();
                match left {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s).unwrap(),
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op),
                };

                write!(f, ", ").unwrap();

                match right {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s).unwrap(),
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op),
                };
                write!(f, ")").unwrap();
            }
            Expression::StringConcat { left, right, .. } => {
                write!(f, "strcat(").unwrap();
                match left {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s).unwrap(),
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op),
                };

                write!(f, ", ").unwrap();

                match right {
                    StringLocation::CompileTime(s) => write!(f, "\"{:?}\"", s).unwrap(),
                    StringLocation::RunTime(op) => self.print_rhs_operand(f, op),
                };
                write!(f, ")").unwrap();
            }
            Expression::StorageArrayLength { array, .. } => {
                write!(f, "storage_arr_len(").unwrap();
                self.print_rhs_operand(f, array);
                write!(f, ")").unwrap();
            }
            Expression::ReturnData { .. } => write!(f, "(extern_call_ret_data)").unwrap(),
            Expression::NumberLiteral { value, .. } => {
                write!(f, "{}", value).unwrap();
            }
            Expression::BoolLiteral { value, .. } => write!(f, "{}", value).unwrap(),
            Expression::Builtin { kind, args, .. } => {
                write!(f, "builtin: {:?}(", kind).unwrap();
                args.iter().enumerate().for_each(|(i, arg)| {
                    if i != 0 {
                        write!(f, ", ").unwrap();
                    }
                    self.print_rhs_operand(f, arg);
                });
                write!(f, ")").unwrap();
            }
            Expression::VectorData { pointer } => {
                write!(f, "ptr_pos(").unwrap();
                self.print_rhs_operand(f, pointer);
                write!(f, ")").unwrap();
            }
        }
    }
}
