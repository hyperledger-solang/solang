// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::StructType;
use crate::sema::ast::{Namespace, Type, Type::Uint};
use solang_parser::pt::Loc::Codegen;
use std::collections::HashMap;

use super::buffer_validator::BufferValidator;

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

/// Decoding the compact integer at current `offset` inside `buffer`.
/// Returns the variable number of the decoded integer (32bit) and the width in bytes of the encoded version.
/// More information can found in the /// [SCALE documentation](https://docs.substrate.io/reference/scale-codec/).
fn decode_compact(
    buffer: &Expression,
    offset: &Expression,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> (usize, Expression) {
    let decoded_var = vartab.temp_anonymous(&Uint(32));
    let size_width_var = vartab.temp_anonymous(&Uint(32));
    vartab.new_dirty_tracker();
    let read_byte = Expression::Builtin(
        Codegen,
        vec![Uint(8)],
        Builtin::ReadFromBuffer,
        vec![buffer.clone(), offset.clone()],
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_width_var,
            expr: Expression::ZeroExt(Codegen, Uint(32), read_byte.into()),
        },
    );
    let size_width = Expression::Variable(Codegen, Uint(32), size_width_var);
    let two = Expression::NumberLiteral(Codegen, Uint(32), 2.into());
    let three = Expression::NumberLiteral(Codegen, Uint(32), 3.into());
    let cond = Expression::BitwiseAnd(Codegen, Uint(32), size_width.clone().into(), three.into());
    let cases = &[
        (
            Expression::NumberLiteral(Codegen, Uint(32), 0.into()),
            cfg.new_basic_block("case_0".into()),
        ),
        (
            Expression::NumberLiteral(Codegen, Uint(32), 1.into()),
            cfg.new_basic_block("case_1".into()),
        ),
        (
            Expression::NumberLiteral(Codegen, Uint(32), 2.into()),
            cfg.new_basic_block("case_2".into()),
        ),
    ];
    let default = cfg.new_basic_block("case_default".into());
    cfg.add(
        vartab,
        Instr::Switch {
            cond,
            cases: cases.to_vec(),
            default,
        },
    );

    let done = cfg.new_basic_block("done".into());
    // We will land in the default block for sizes of 2**30 (1GB) or larger.
    // Such big sizes are invalid for smart contracts and should never occur anyways.
    cfg.set_basic_block(default);
    cfg.add(vartab, Instr::AssertFailure { encoded_args: None });

    cfg.set_basic_block(cases[0].1);
    let expr = Expression::ShiftRight(
        Codegen,
        Uint(32),
        size_width.clone().into(),
        two.clone().into(),
        false,
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: decoded_var,
            expr,
        },
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_width_var,
            expr: Expression::NumberLiteral(Codegen, Uint(32), 1.into()),
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(cases[1].1);
    let read_byte = Expression::Builtin(
        Codegen,
        vec![Uint(16)],
        Builtin::ReadFromBuffer,
        vec![buffer.clone(), offset.clone()],
    );
    let expr = Expression::ShiftRight(
        Codegen,
        Uint(32),
        Expression::ZeroExt(Codegen, Uint(32), read_byte.into()).into(),
        two.clone().into(),
        false,
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: decoded_var,
            expr,
        },
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_width_var,
            expr: two.clone(),
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(cases[2].1);
    let read_byte = Expression::Builtin(
        Codegen,
        vec![Uint(32)],
        Builtin::ReadFromBuffer,
        vec![buffer.clone(), offset.clone()],
    );
    let expr = Expression::ShiftRight(Codegen, Uint(32), read_byte.into(), two.into(), false);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: decoded_var,
            expr,
        },
    );
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_width_var,
            expr: Expression::NumberLiteral(Codegen, Uint(32), 4.into()),
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    vartab.set_dirty(decoded_var);
    vartab.set_dirty(size_width_var);

    cfg.set_basic_block(done);
    cfg.set_phis(done, vartab.pop_dirty_tracker());

    (decoded_var, size_width)
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
    let cmp_val = Expression::NumberLiteral(Codegen, Uint(32), (0x40000000 - 1).into());
    let compare = Expression::More {
        loc: Codegen,
        signed: false,
        left: expr.clone().into(),
        right: cmp_val.into(),
    };
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
    let cmp_val = Expression::NumberLiteral(Codegen, Uint(32), (0x40 - 1).into());
    let compare = Expression::More {
        loc: Codegen,
        signed: false,
        left: expr.clone().into(),
        right: cmp_val.into(),
    };
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: compare,
            true_block: medium_or_big,
            false_block: small,
        },
    );

    cfg.set_basic_block(medium_or_big);
    let cmp_val = Expression::NumberLiteral(Codegen, Uint(32), (0x4000 - 1).into());
    let compare = Expression::More {
        loc: Codegen,
        signed: false,
        left: expr.clone().into(),
        right: cmp_val.into(),
    };
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: compare,
            true_block: big,
            false_block: medium,
        },
    );
    let size_variable = vartab.temp_anonymous(&Uint(32));
    vartab.new_dirty_tracker();
    let four = Expression::NumberLiteral(Codegen, Uint(32), 4.into()).into();
    let mul = Expression::Multiply(Codegen, Uint(32), false, expr.clone().into(), four);

    cfg.set_basic_block(small);
    if let (Some(buffer), Some(offset)) = (buffer, offset) {
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: Expression::Cast(Codegen, Uint(8), mul.clone().into()),
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
        let mul = Expression::BitwiseOr(Codegen, Uint(32), mul.clone().into(), one.into());
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: Expression::Cast(Codegen, Uint(16), mul.into()),
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
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: Expression::BitwiseOr(Codegen, Uint(32), mul.into(), two.into()),
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
        // FIXME:
        // It should be possible to optimize this to estimate always 4 bytes.
        // `codegen::abi_encode()` also returns the actual encoded size,
        // so slightly overestimating it shouldn't  matter.
        // However, the actual length of the encoded data produced by `codegen::abi_encode()`
        // is ignored in some places, wich results in buggy contracts if we have not an exact estimate.
        // Once this is fixed (the encoded size return by `codegen::abi_encode()` must never be ignored),
        // this can just be always 4 bytes .
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
        let size = self.encode_directly(&address, buffer, offset, vartab, cfg, addr_len);
        let offset = Expression::Add(Codegen, Uint(32), false, offset.clone().into(), size.into());
        let selector = expr.external_function_selector();
        self.encode_directly(&selector, buffer, &offset, vartab, cfg, 4.into());
        Expression::NumberLiteral(Codegen, Uint(32), (ns.address_length + 4).into())
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

    fn decode_external_function(
        &self,
        buffer: &Expression,
        offset: &Expression,
        ty: &Type,
        validator: &mut BufferValidator,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (Expression, Expression) {
        let size = Expression::NumberLiteral(Codegen, Uint(32), (ns.address_length + 4).into());
        validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);
        let address = Expression::Builtin(
            Codegen,
            vec![Type::Address(false)],
            Builtin::ReadFromBuffer,
            vec![buffer.clone(), offset.clone()],
        );
        let new_offset = offset.clone().add_u32(Expression::NumberLiteral(
            Codegen,
            Uint(32),
            ns.address_length.into(),
        ));
        let selector = Expression::Builtin(
            Codegen,
            vec![Type::FunctionSelector],
            Builtin::ReadFromBuffer,
            vec![buffer.clone(), new_offset],
        );
        let ext_func = Expression::StructLiteral(
            Codegen,
            Type::Struct(StructType::ExternalFunction),
            vec![selector, address],
        );
        (Expression::Cast(Codegen, ty.clone(), ext_func.into()), size)
    }

    fn retrieve_array_length(
        &self,
        buffer: &Expression,
        offset: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> (usize, Expression) {
        decode_compact(buffer, offset, vartab, cfg)
    }

    fn storage_cache_insert(&mut self, arg_no: usize, expr: Expression) {
        self.storage_cache.insert(arg_no, expr);
    }

    fn storage_cache_remove(&mut self, arg_no: usize) -> Option<Expression> {
        self.storage_cache.remove(&arg_no)
    }

    fn calculate_string_size(
        &self,
        expr: &Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Expression {
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
            encode_compact(&length, None, None, vartab, cfg).add_u32(length)
        }
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }
}
