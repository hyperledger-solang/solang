// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::ast::{self, Function, Namespace, RetrieveType, StringLocation, Type};
use ink_env::hash::{Blake2x256, CryptoHash};
use parity_scale_codec as scale;
use scale::Encode;
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
        let mut data = Vec::new();
        let mut data_tys = Vec::new();
        let mut topics = Vec::new();
        let mut topic_tys = Vec::new();

        let loc = pt::Loc::Builtin;
        let event = &self.ns.events[self.event_no];
        let hash_len = || Box::new(Expression::NumberLiteral(loc, Type::Uint(32), 32.into()));

        // Events that are not anonymous always have themselves as a topic.
        // This is static and can be calculated at compile time.
        if !event.anonymous {
            let mut encoded =
                format!("{}::{}", &self.ns.contracts[contract_no].name, &event.name).encode();
            encoded[0] = 0; // Set the prefix (there is no prefix for the event topic)
            topics.push(Expression::AllocDynamicBytes(
                loc,
                Type::Slice(Type::Uint(8).into()),
                hash_len(),
                Some(topic_hash(&encoded[..])),
            ));
            topic_tys.push(Type::DynamicBytes);
        };

        // Topic prefixes are static and can be calculated at compile time.
        let mut topic_prefixes: Vec<Vec<u8>> = event
            .fields
            .iter()
            .filter(|field| field.indexed)
            .map(|field| {
                format!(
                    "{}::{}::{}",
                    &self.ns.contracts[contract_no].name,
                    &event.name,
                    &field.name_as_str()
                )
                .into_bytes()
                .encode()
            })
            .collect();

        for (ast_exp, field) in self.args.iter().zip(event.fields.iter()) {
            let value_exp = expression(ast_exp, cfg, contract_no, Some(func), self.ns, vartab, opt);
            data_tys.push(value_exp.ty());
            data.push(value_exp);

            if !field.indexed {
                continue;
            }

            let value_exp = expression(ast_exp, cfg, contract_no, Some(func), self.ns, vartab, opt);
            let encoded = Expression::AbiEncode {
                loc,
                tys: vec![value_exp.ty()],
                packed: vec![],
                args: vec![value_exp],
            };
            let concatenated = Expression::StringConcat(
                loc,
                Type::DynamicBytes,
                StringLocation::CompileTime(topic_prefixes.remove(0)), // TODO not efficient
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
                hash_len(),
            );

            let bigger = cfg.new_basic_block("bigger".into());
            let done = cfg.new_basic_block("done".into());
            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: compare,
                    true_block: bigger,
                    false_block: done,
                },
            );

            cfg.set_basic_block(bigger);
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
            cfg.add(vartab, Instr::Branch { block: done });

            cfg.set_basic_block(done);
            cfg.set_phis(done, vartab.pop_dirty_tracker());

            topic_tys.push(Type::DynamicBytes);
            topics.push(buffer);
        }

        cfg.add(
            vartab,
            Instr::EmitEvent {
                event_no: self.event_no,
                data,
                data_tys,
                topics,
                topic_tys,
            },
        );
    }
}
