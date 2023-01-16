// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::{increment_by, AbiEncoding};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{Namespace, Parameter, Type, Type::Uint};
use solang_parser::pt::{Loc, Loc::Codegen};
use std::collections::HashMap;

pub(super) struct ScaleEncoding {
    storage_cache: HashMap<usize, Expression>,
    packed_encoder: bool,
}

impl ScaleEncoding {
    pub fn new(packed: bool) -> Self {
        Self {
            storage_cache: HashMap::new(),
            packed_encoder: packed,
        }
    }
}

/// Encode `expr` into `buffer` as a compact integer. More information can found in the
/// [SCALE documentation](https://docs.substrate.io/reference/scale-codec/).
fn encode_compact(
    expr: &Expression,
    buffer: Option<&Expression>,
    offset: Option<&Expression>,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> Expression {
    let small = cfg.new_basic_block("small".into());
    let medium = cfg.new_basic_block("medium".into());
    let medium_or_big = cfg.new_basic_block("medium_or_big".into());
    let big = cfg.new_basic_block("big".into());
    let done = cfg.new_basic_block("done".into());
    let fail = cfg.new_basic_block("fail".into());
    let prepare = cfg.new_basic_block("prepare".into());
    let cmp_val = Expression::NumberLiteral(Codegen, Uint(32), 0x40000000.into());
    let compare = Expression::UnsignedMore(Codegen, expr.clone().into(), cmp_val.into());
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: compare,
            true_block: fail,
            false_block: prepare,
        },
    );

    cfg.set_basic_block(fail);
    cfg.add(vartab, Instr::AssertFailure { encoded_args: None });

    cfg.set_basic_block(prepare);
    let cmp_val = Expression::NumberLiteral(Codegen, Uint(32), 0x40.into());
    let compare = Expression::UnsignedMore(Codegen, expr.clone().into(), cmp_val.into());
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: compare,
            true_block: medium_or_big,
            false_block: small,
        },
    );

    cfg.set_basic_block(medium_or_big);
    let cmp_val = Expression::NumberLiteral(Codegen, Uint(32), 0x4000.into());
    let compare = Expression::UnsignedMore(Codegen, expr.clone().into(), cmp_val.into());
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: compare,
            true_block: big,
            false_block: medium,
        },
    );
    vartab.new_dirty_tracker();
    let size_variable = vartab.temp_anonymous(&Uint(32));
    let four = Expression::NumberLiteral(Codegen, Uint(32), 4.into()).into();
    let mul = Expression::Multiply(Codegen, Uint(32), false, expr.clone().into(), four);

    cfg.set_basic_block(small);
    if let (Some(buffer), Some(offset)) = (buffer, offset) {
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: mul.clone(),
            },
        );
    }
    let one = Expression::NumberLiteral(Codegen, Uint(32), 1.into());
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_variable,
            expr: one.clone(),
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(medium);
    if let (Some(buffer), Some(offset)) = (buffer, offset) {
        let mul2 = Expression::BitwiseOr(Codegen, Uint(32), mul.clone().into(), one.into());
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: mul2,
            },
        );
    }
    let two = Expression::NumberLiteral(Codegen, Uint(32), 2.into());
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_variable,
            expr: two.clone(),
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(big);
    if let (Some(buffer), Some(offset)) = (buffer, offset) {
        let mul2 = Expression::BitwiseOr(Codegen, Uint(32), mul.into(), two.into());
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: mul2,
            },
        );
    }
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_variable,
            expr: Expression::NumberLiteral(Codegen, Uint(32), 4.into()),
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(done);
    cfg.set_phis(done, vartab.pop_dirty_tracker());
    Expression::Variable(Codegen, Uint(32), size_variable)
}

impl AbiEncoding for ScaleEncoding {
    fn size_width(
        &self,
        size: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        encode_compact(size, None, None, vartab, cfg)
    }

    fn encode_external_function(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        let addr_len = ns.address_length.into();
        let address = expr.external_function_address();
        let size = self.encode_linear(&address, buffer, offset, vartab, cfg, addr_len);
        let offset = Expression::Add(
            Codegen,
            Uint(32),
            false,
            offset.clone().into(),
            size.clone().into(),
        );
        let selector = expr.external_function_selector();
        let selector_size = self.encode_linear(&selector, buffer, &offset, vartab, cfg, 4.into());
        Expression::Add(Codegen, Uint(32), false, size.into(), selector_size.into())
    }

    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        encode_compact(expr, Some(buffer), Some(offset), vartab, cfg)
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

    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
    }

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression> {
        self.storage_cache.remove(&arg_no)
    }

    fn get_encoding_size(
        &self,
        expr: &Expression,
        ty: &Type,
        _ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
        match ty {
            Type::String | Type::DynamicBytes => {
                // When encoding a variable length array, the total size is "compact encoded array length + N elements"
                let length = Expression::Builtin(
                    Codegen,
                    vec![Uint(32)],
                    Builtin::ArrayLength,
                    vec![expr.clone()],
                );
                if self.is_packed() {
                    length
                } else {
                    increment_by(encode_compact(&length, None, None, vartab, cfg), length)
                }
            }
            _ => unreachable!("Type should have the same size for all encoding schemes"),
        }
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }
}
