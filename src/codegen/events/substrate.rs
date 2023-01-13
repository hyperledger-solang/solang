// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;
use std::vec;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::abi_encode;
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::ast::{self, Function, Namespace, RetrieveType, StringLocation, Type};
use ink::env::hash::{Blake2x256, CryptoHash};
use parity_scale_codec::Encode;
use solang_parser::pt;

/// This struct implements the trait 'EventEmitter' in order to handle the emission of events
/// for Substrate
pub(super) struct SubstrateEventEmitter<'a> {
    /// Arguments passed to the event
    pub(super) args: &'a [ast::Expression],
    pub(super) ns: &'a Namespace,
    pub(super) event_no: usize,
}

/// Takes a scale-encoded topic and makes it into a topic hash.
fn topic_hash(encoded: &[u8]) -> Vec<u8> {
    let mut buf = [0; 32];
    if encoded.len() <= 32 {
        buf[..encoded.len()].copy_from_slice(encoded);
    } else {
        <Blake2x256 as CryptoHash>::hash(encoded, &mut buf);
    };
    buf.into()
}

impl EventEmitter for SubstrateEventEmitter<'_> {
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
        // For freestanding events the name of the emitting contract is used
        let contract_name = &self.ns.contracts[event.contract.unwrap_or(contract_no)].name;
        let hash_len = Box::new(Expression::NumberLiteral(loc, Type::Uint(32), 32.into()));
        let id = self.ns.contracts[contract_no]
            .emits_events
            .iter()
            .position(|e| *e == self.event_no)
            .expect("contract emits this event");
        let mut data = vec![Expression::NumberLiteral(loc, Type::Uint(8), id.into())];
        let (mut topics, mut topic_tys) = (vec![], vec![]);

        // Events that are not anonymous always have themselves as a topic.
        // This is static and can be calculated at compile time.
        if !event.anonymous {
            // First byte is 0 because there is no prefix for the event topic
            let encoded = format!("\0{}::{}", contract_name, &event.name);
            topics.push(Expression::AllocDynamicBytes(
                loc,
                Type::Slice(Type::Uint(8).into()),
                hash_len.clone(),
                Some(topic_hash(encoded.as_bytes())),
            ));
            topic_tys.push(Type::DynamicBytes);
        };

        // Topic prefixes are static and can be calculated at compile time.
        let mut topic_prefixes: VecDeque<Vec<u8>> = event
            .fields
            .iter()
            .filter(|field| field.indexed)
            .map(|field| {
                format!(
                    "{}::{}::{}",
                    contract_name,
                    &event.name,
                    &field.name_as_str()
                )
                .into_bytes()
                .encode()
            })
            .collect();

        for (ast_exp, field) in self.args.iter().zip(event.fields.iter()) {
            let value_exp = expression(ast_exp, cfg, contract_no, Some(func), self.ns, vartab, opt);
            let value_var = vartab.temp_anonymous(&value_exp.ty());
            let value = Expression::Variable(loc, value_exp.ty(), value_var);
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

            let encoded = Expression::AbiEncode {
                loc,
                tys: vec![value.ty()],
                packed: vec![],
                args: vec![value],
            };
            let concatenated = Expression::StringConcat(
                loc,
                Type::DynamicBytes,
                StringLocation::CompileTime(topic_prefixes.pop_front().unwrap()),
                StringLocation::RunTime(encoded.into()),
            );

            vartab.new_dirty_tracker();
            let var_buffer = vartab.temp_anonymous(&Type::DynamicBytes);
            cfg.add(
                vartab,
                Instr::Set {
                    loc,
                    res: var_buffer,
                    expr: concatenated,
                },
            );
            let buffer = Expression::Variable(loc, Type::DynamicBytes, var_buffer);
            let compare = Expression::UnsignedMore(
                loc,
                Expression::Builtin(
                    loc,
                    vec![Type::Uint(32)],
                    Builtin::ArrayLength,
                    vec![buffer.clone()],
                )
                .into(),
                hash_len.clone(),
            );

            let hash_topic_block = cfg.new_basic_block("hash_topic".into());
            let done_block = cfg.new_basic_block("done".into());
            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: compare,
                    true_block: hash_topic_block,
                    false_block: done_block,
                },
            );

            cfg.set_basic_block(hash_topic_block);
            cfg.add(
                vartab,
                Instr::WriteBuffer {
                    buf: buffer.clone(),
                    offset: Expression::NumberLiteral(loc, Type::Uint(32), 0.into()),
                    value: Expression::Builtin(
                        loc,
                        vec![Type::Bytes(32)],
                        Builtin::Blake2_256,
                        vec![buffer.clone()],
                    ),
                },
            );
            vartab.set_dirty(var_buffer);
            cfg.add(vartab, Instr::Branch { block: done_block });

            cfg.set_basic_block(done_block);
            cfg.set_phis(done_block, vartab.pop_dirty_tracker());

            topic_tys.push(Type::DynamicBytes);
            topics.push(buffer);
        }

        let (data, size) = abi_encode(&loc, data, self.ns, vartab, cfg, false);
        cfg.add(
            vartab,
            Instr::EmitEvent {
                event_no: self.event_no,
                data: vec![data, size],
                data_tys: vec![],
                topics,
                topic_tys,
            },
        );
    }
}
