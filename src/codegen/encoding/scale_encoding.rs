// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{Namespace, Parameter, RetrieveType, Type};
use num_bigint::BigInt;
use solang_parser::pt::Loc;

use super::calculate_size_args;

/// This struct implements the trait AbiEncoding for Parity's Scale encoding
pub(super) struct ScaleEncoding {
    /// Are we pakced encoding?
    packed_encoder: bool,
}

impl ScaleEncoding {
    pub fn new(packed: bool) -> ScaleEncoding {
        ScaleEncoding {
            packed_encoder: packed,
        }
    }
}

impl AbiEncoding for ScaleEncoding {
    fn abi_encode(
        &mut self,
        loc: &Loc,
        mut args: Vec<Expression>,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        //let size = calculate_size_args(self, &args, ns, vartab, cfg);
        let size = Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), (32 * 1024).into());

        let encoded_bytes = vartab.temp_name("abi_encoded", &Type::DynamicBytes);
        cfg.add(
            vartab,
            Instr::Set {
                loc: *loc,
                res: encoded_bytes,
                expr: Expression::AllocDynamicBytes(
                    *loc,
                    Type::DynamicBytes,
                    Box::new(size.clone()),
                    None,
                ),
            },
        );

        let mut offset = Expression::NumberLiteral(*loc, Type::Uint(32), 0.into());
        let buffer = Expression::Variable(*loc, Type::DynamicBytes, encoded_bytes);

        for (arg_no, item) in args.iter().enumerate() {
            let advance = self.encode(item, &buffer, &offset, arg_no, ns, vartab, cfg);
            offset = Expression::Add(
                Loc::Codegen,
                Type::Uint(32),
                false,
                Box::new(offset),
                Box::new(advance),
            );
        }

        (buffer, size)

        //let tys = args.iter().map(|e| e.ty()).collect::<Vec<Type>>();

        //let encoded_buffer = vartab.temp_anonymous(&Type::DynamicBytes);
        //let mut packed: Vec<Expression> = Vec::new();
        //if self.packed_encoder {
        //    std::mem::swap(&mut packed, &mut args);
        //}

        //cfg.add(
        //    vartab,
        //    Instr::Set {
        //        loc: *loc,
        //        res: encoded_buffer,
        //        expr: Expression::AbiEncode {
        //            loc: *loc,
        //            packed,
        //            args,
        //            tys,
        //        },
        //    },
        //);

        //let encoded_expr = Expression::Variable(*loc, Type::DynamicBytes, encoded_buffer);
        //let buffer_len = Expression::Builtin(
        //    *loc,
        //    vec![Type::Uint(32)],
        //    Builtin::ArrayLength,
        //    vec![encoded_expr.clone()],
        //);

        //(encoded_expr, buffer_len)
    }

    fn abi_decode(
        &self,
        loc: &Loc,
        buffer: &Expression,
        types: &[Type],
        _ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
        buffer_size: Option<Expression>,
    ) -> Vec<Expression> {
        assert!(!self.packed_encoder);
        let mut returns: Vec<Expression> = Vec::with_capacity(types.len());
        let mut var_nos: Vec<usize> = Vec::with_capacity(types.len());
        let mut decode_params: Vec<Parameter> = Vec::with_capacity(types.len());

        for item in types {
            let var_no = vartab.temp_anonymous(item);
            var_nos.push(var_no);
            returns.push(Expression::Variable(*loc, item.clone(), var_no));
            decode_params.push(Parameter {
                loc: Loc::Codegen,
                id: None,
                ty: item.clone(),
                ty_loc: None,
                indexed: false,
                readonly: false,
                recursive: false,
            });
        }

        cfg.add(
            vartab,
            Instr::AbiDecode {
                res: var_nos,
                selector: None,
                exception_block: None,
                tys: decode_params,
                data: buffer.clone(),
                data_len: buffer_size,
            },
        );

        returns
    }

    fn cache_storage_loaded(&mut self, _arg_no: usize, _expr: Expression) {
        unreachable!("This function is not needed for Scale encoding");
    }

    fn get_encoding_size(&self, _expr: &Expression, _ty: &Type, _ns: &Namespace) -> Expression {
        unreachable!("This function is not needed for Scale encoding");
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }
}

impl ScaleEncoding {
    fn encode(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        arg_no: usize,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let expr_ty = expr.ty().unwrap_user_type(ns);
        let mut size = 0.into();

        match &expr.ty() {
            Type::Bool => {
                cfg.add(
                    vartab,
                    Instr::WriteBuffer {
                        buf: buffer.clone(),
                        offset: offset.clone(),
                        value: expr.clone(),
                    },
                );
                size = 1.into()
            }
            _ => todo!(),
        }

        Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), size)
    }
}
