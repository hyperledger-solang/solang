// SPDX-License-Identifier: Apache-2.0

use std::vec;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::abi_encode;
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::ast::{self, Function, Namespace, RetrieveType, Type};
use ink_env::hash::{Blake2x256, CryptoHash};
use solang_parser::pt;

/// Implements [EventEmitter] to handle the emission of events on Polkadot.
/// Data and topic encoding follow [ink! v5.0][0].
///
/// [0]: https://use.ink/basics/events/#topics
pub(super) struct PolkadotEventEmitter<'a> {
    /// Arguments passed to the event
    pub(super) args: &'a [ast::Expression],
    pub(super) ns: &'a Namespace,
    pub(super) event_no: usize,
}

impl EventEmitter for PolkadotEventEmitter<'_> {
    fn selector(&self, _emitting_contract_no: usize) -> Vec<u8> {
        let signature = self.ns.events[self.event_no].signature.as_bytes();
        let mut buf = [0; 32];
        <Blake2x256 as CryptoHash>::hash(signature, &mut buf);
        buf.into()
    }

    fn emit(
        &self,
        contract_no: usize,
        func: &Function,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        opt: &Options,
    ) {
        let loc = pt::Loc::Builtin;
        let event = &self.ns.events[self.event_no];
        let hash_len = Box::new(Expression::NumberLiteral {
            loc,
            ty: Type::Uint(32),
            value: 32.into(),
        });
        let (mut data, mut topics) = (Vec::new(), Vec::new());

        // Events that are not anonymous always have themselves as a topic.
        // This is static and can be calculated at compile time.
        if !event.anonymous {
            topics.push(Expression::AllocDynamicBytes {
                loc,
                ty: Type::Slice(Type::Uint(8).into()),
                size: hash_len.clone(),
                initializer: self.selector(contract_no).into(),
            });
        };

        for (ast_exp, field) in self.args.iter().zip(event.fields.iter()) {
            let value_exp = expression(ast_exp, cfg, contract_no, Some(func), self.ns, vartab, opt);
            let value_var = vartab.temp_anonymous(&value_exp.ty());
            let value = Expression::Variable {
                loc,
                ty: value_exp.ty(),
                var_no: value_var,
            };
            cfg.add(
                vartab,
                Instr::Set {
                    loc,
                    res: value_var,
                    expr: value_exp,
                },
            );
            data.push(value.clone());

            if !field.indexed {
                continue;
            }

            let (value_encoded, size) = abi_encode(&loc, vec![value], self.ns, vartab, cfg, false);

            vartab.new_dirty_tracker();
            let var_buffer = vartab.temp_anonymous(&Type::DynamicBytes);
            cfg.add(
                vartab,
                Instr::Set {
                    loc,
                    res: var_buffer,
                    expr: value_encoded,
                },
            );
            let buffer = Expression::Variable {
                loc,
                ty: Type::DynamicBytes,
                var_no: var_buffer,
            };

            let hash_topic_block = cfg.new_basic_block("hash_topic".into());
            let done_block = cfg.new_basic_block("done".into());
            let size_is_greater_than_hash_length = Expression::More {
                loc,
                signed: false,
                left: size.clone().into(),
                right: hash_len.clone(),
            };
            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: size_is_greater_than_hash_length,
                    true_block: hash_topic_block,
                    false_block: done_block,
                },
            );

            cfg.set_basic_block(hash_topic_block);
            cfg.add(
                vartab,
                Instr::WriteBuffer {
                    buf: buffer.clone(),
                    offset: Expression::NumberLiteral {
                        loc,
                        ty: Type::Uint(32),
                        value: 0.into(),
                    },
                    value: Expression::Builtin {
                        loc,
                        tys: vec![Type::Bytes(32)],
                        kind: Builtin::Blake2_256,
                        args: vec![buffer.clone()],
                    },
                },
            );
            vartab.set_dirty(var_buffer);
            cfg.add(vartab, Instr::Branch { block: done_block });

            cfg.set_basic_block(done_block);
            cfg.set_phis(done_block, vartab.pop_dirty_tracker());

            topics.push(buffer);
        }

        let data = self
            .args
            .iter()
            .map(|e| expression(e, cfg, contract_no, Some(func), self.ns, vartab, opt))
            .collect::<Vec<_>>();
        let encoded_data = data
            .is_empty()
            .then(|| Expression::AllocDynamicBytes {
                loc,
                ty: Type::DynamicBytes,
                size: Expression::NumberLiteral {
                    loc,
                    ty: Type::Uint(32),
                    value: 0.into(),
                }
                .into(),
                initializer: Vec::new().into(),
            })
            .unwrap_or_else(|| abi_encode(&loc, data, self.ns, vartab, cfg, false).0);

        cfg.add(
            vartab,
            Instr::EmitEvent {
                event_no: self.event_no,
                data: encoded_data,
                topics,
            },
        );
    }
}
