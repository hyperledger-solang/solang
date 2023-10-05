// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::AbiEncoding;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::StructType;
use crate::sema::ast::{Namespace, Type, Type::Uint};
use parity_scale_codec::Encode;
use primitive_types::U256;
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
    let read_byte = Expression::Builtin {
        loc: Codegen,
        tys: vec![Uint(8)],
        kind: Builtin::ReadFromBuffer,
        args: vec![buffer.clone(), offset.clone()],
    };
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_width_var,
            expr: Expression::ZeroExt {
                loc: Codegen,
                ty: Uint(32),
                expr: read_byte.into(),
            },
        },
    );
    let size_width = Expression::Variable {
        loc: Codegen,
        ty: Uint(32),
        var_no: size_width_var,
    };
    let two = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: 2.into(),
    };
    let three = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: 3.into(),
    };
    let cond = Expression::BitwiseAnd {
        loc: Codegen,
        ty: Uint(32),
        left: size_width.clone().into(),
        right: three.into(),
    };
    let cases = &[
        (
            Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 0.into(),
            },
            cfg.new_basic_block("case_0".into()),
        ),
        (
            Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 1.into(),
            },
            cfg.new_basic_block("case_1".into()),
        ),
        (
            Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 2.into(),
            },
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
    let expr = Expression::ShiftRight {
        loc: Codegen,
        ty: Uint(32),
        left: size_width.clone().into(),
        right: two.clone().into(),
        signed: false,
    };
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
            expr: Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 1.into(),
            },
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(cases[1].1);
    let read_byte = Expression::Builtin {
        loc: Codegen,
        tys: vec![Uint(16)],
        kind: Builtin::ReadFromBuffer,
        args: vec![buffer.clone(), offset.clone()],
    };
    let expr = Expression::ShiftRight {
        loc: Codegen,
        ty: Uint(32),
        left: Expression::ZeroExt {
            loc: Codegen,
            ty: Uint(32),
            expr: read_byte.into(),
        }
        .into(),
        right: two.clone().into(),
        signed: false,
    };
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
    let read_byte = Expression::Builtin {
        loc: Codegen,
        tys: vec![Uint(32)],
        kind: Builtin::ReadFromBuffer,
        args: vec![buffer.clone(), offset.clone()],
    };
    let expr = Expression::ShiftRight {
        loc: Codegen,
        ty: Uint(32),
        left: read_byte.into(),
        right: two.into(),
        signed: false,
    };
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
            expr: Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 4.into(),
            },
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
    let cmp_val = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: (0x40000000 - 1).into(),
    };
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
    let cmp_val = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: (0x40 - 1).into(),
    };
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
    let cmp_val = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: (0x4000 - 1).into(),
    };
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
    let four = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: 4.into(),
    }
    .into();
    let mul = Expression::Multiply {
        loc: Codegen,
        ty: Uint(32),
        overflowing: false,
        left: expr.clone().into(),
        right: four,
    };

    cfg.set_basic_block(small);
    if let (Some(buffer), Some(offset)) = (buffer, offset) {
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: Expression::Trunc {
                    loc: Codegen,
                    ty: Uint(8),
                    expr: mul.clone().into(),
                },
            },
        );
    }
    let one = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: 1.into(),
    };
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
        let mul = Expression::BitwiseOr {
            loc: Codegen,
            ty: Uint(32),
            left: mul.clone().into(),
            right: one.into(),
        };
        cfg.add(
            vartab,
            Instr::WriteBuffer {
                buf: buffer.clone(),
                offset: offset.clone(),
                value: Expression::Trunc {
                    loc: Codegen,
                    ty: Uint(16),
                    expr: mul.into(),
                },
            },
        );
    }
    let two = Expression::NumberLiteral {
        loc: Codegen,
        ty: Uint(32),
        value: 2.into(),
    };
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
                value: Expression::BitwiseOr {
                    loc: Codegen,
                    ty: Uint(32),
                    left: mul.into(),
                    right: two.into(),
                },
            },
        );
    }
    cfg.add(
        vartab,
        Instr::Set {
            loc: Codegen,
            res: size_variable,
            expr: Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 4.into(),
            },
        },
    );
    cfg.add(vartab, Instr::Branch { block: done });

    cfg.set_basic_block(done);
    cfg.set_phis(done, vartab.pop_dirty_tracker());
    Expression::Variable {
        loc: Codegen,
        ty: Uint(32),
        var_no: size_variable,
    }
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
        let offset = Expression::Add {
            loc: Codegen,
            ty: Uint(32),
            overflowing: false,
            left: offset.clone().into(),
            right: size.into(),
        };
        let selector = expr.external_function_selector();
        self.encode_directly(&selector, buffer, &offset, vartab, cfg, 4.into());
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: (ns.address_length + 4).into(),
        }
    }

    fn encode_size(
        &mut self,
        expr: &Expression,
        buffer: &Expression,
        offset: &Expression,
        _ns: &Namespace,
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
        let size = Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: (ns.address_length + 4).into(),
        };
        validator.validate_offset_plus_size(offset, &size, ns, vartab, cfg);
        let address = Expression::Builtin {
            loc: Codegen,
            tys: vec![Type::Address(false)],
            kind: Builtin::ReadFromBuffer,
            args: vec![buffer.clone(), offset.clone()],
        };
        let new_offset = offset.clone().add_u32(Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: ns.address_length.into(),
        });
        let selector = Expression::Builtin {
            loc: Codegen,
            tys: vec![Type::FunctionSelector],
            kind: Builtin::ReadFromBuffer,
            args: vec![buffer.clone(), new_offset],
        };
        let ext_func = Expression::StructLiteral {
            loc: Codegen,
            ty: Type::Struct(StructType::ExternalFunction),
            values: vec![selector, address],
        };
        (
            Expression::Cast {
                loc: Codegen,
                ty: ty.clone(),
                expr: ext_func.into(),
            },
            size,
        )
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
        let length = Expression::Builtin {
            loc: Codegen,
            tys: vec![Uint(32)],
            kind: Builtin::ArrayLength,
            args: vec![expr.clone()],
        };
        if self.is_packed() {
            length
        } else {
            encode_compact(&length, None, None, vartab, cfg).add_u32(length)
        }
    }

    fn is_packed(&self) -> bool {
        self.packed_encoder
    }

    /// TODO: This is used and tested for error data (Error and Panic) only.
    fn const_encode(&self, args: &[Expression]) -> Option<Vec<u8>> {
        let mut result = vec![];
        for arg in args {
            match arg {
                Expression::AllocDynamicBytes {
                    initializer: Some(data),
                    ty: Type::String | Type::DynamicBytes,
                    ..
                } => result.extend_from_slice(&data.encode()),
                Expression::AllocDynamicBytes {
                    initializer: Some(data),
                    ty: Type::Slice(inner),
                    ..
                } if matches!(**inner, Type::Bytes(1)) => result.extend_from_slice(data),
                Expression::NumberLiteral {
                    ty: Type::Bytes(4),
                    value,
                    ..
                } => {
                    let bytes = value.to_bytes_be().1;
                    if bytes.len() < 4 {
                        let buf = vec![0; 4 - bytes.len()];
                        result.extend_from_slice(&buf);
                    }
                    result.extend_from_slice(&bytes[..]);
                }
                Expression::NumberLiteral {
                    ty: Type::Uint(256),
                    value,
                    ..
                } => {
                    let bytes = value.to_bytes_be().1;
                    result.extend_from_slice(&U256::from_big_endian(&bytes).encode()[..]);
                }
                _ => return None,
            }
        }
        result.into()
    }
}

#[cfg(test)]
mod tests {
    use num_bigint::{BigInt, Sign};
    use parity_scale_codec::Encode;
    use primitive_types::U256;

    use crate::{
        codegen::{
            encoding::{scale_encoding::ScaleEncoding, AbiEncoding},
            Expression,
        },
        sema::ast::Type,
    };

    #[test]
    fn const_encode_dynamic_bytes() {
        let data = vec![0x41, 0x41];
        let encoder = ScaleEncoding::new(false);
        let expr = Expression::AllocDynamicBytes {
            loc: Default::default(),
            ty: Type::DynamicBytes,
            size: Expression::Poison.into(),
            initializer: data.clone().into(),
        };
        let encoded = encoder.const_encode(&[expr]).unwrap();
        assert_eq!(encoded, data.encode());
    }

    #[test]
    fn const_encode_uint() {
        let encoder = ScaleEncoding::new(false);
        for value in [U256::MAX, U256::zero(), U256::one()] {
            let mut bytes = [0u8; 32].to_vec();
            value.to_big_endian(&mut bytes);
            let data = BigInt::from_bytes_be(Sign::Plus, &bytes);
            let expr = Expression::NumberLiteral {
                loc: Default::default(),
                ty: Type::Uint(256),
                value: data,
            };
            let encoded = encoder.const_encode(&[expr]).unwrap();
            assert_eq!(encoded, value.encode());
        }
    }

    #[test]
    fn const_encode_bytes4() {
        let encoder = ScaleEncoding::new(false);
        for value in [
            [0x00, 0x00, 0xff, 0xff],
            [0x00, 0xff, 0xff, 0x00],
            [0xff, 0xff, 0x00, 0x00],
            [0xff, 0xff, 0xff, 0xff],
            [0x00, 0x00, 0x00, 0x00],
            [0xde, 0xad, 0xbe, 0xef],
            [0x01, 0x00, 0x00, 0x00],
            [0x00, 0x00, 0x00, 0x01],
        ] {
            let expr = Expression::NumberLiteral {
                ty: Type::Bytes(4),
                value: BigInt::from_bytes_be(Sign::Plus, &value),
                loc: Default::default(),
            };
            assert_eq!(&encoder.const_encode(&[expr]).unwrap(), &value.encode());
        }
    }
}
