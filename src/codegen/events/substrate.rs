// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::ast::{self, ArrayLength};
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use ink_env::hash::{Blake2x256, CryptoHash, HashOutput};
use ink_env::topics::PrefixedValue;
use ink_primitives::{Clear, Hash};
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

/// Taken from the ink erc20 example test
pub(crate) fn encoded_into_hash<T>(entity: &T) -> Hash
where
    T: scale::Encode,
{
    let mut result = Hash::clear();
    let len_result = result.as_ref().len();
    let encoded = entity.encode();
    let len_encoded = encoded.len();
    if len_encoded <= len_result {
        result.as_mut()[..len_encoded].copy_from_slice(&encoded);
        return result;
    }
    let mut hash_output = <<Blake2x256 as HashOutput>::Type as Default>::default();
    <Blake2x256 as CryptoHash>::hash(&encoded, &mut hash_output);
    let copy_len = core::cmp::min(hash_output.len(), len_result);
    result.as_mut()[0..copy_len].copy_from_slice(&hash_output[0..copy_len]);
    result
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
        let topic_ty = || Type::DynamicBytes;

        // Events that are not anonymous always have themselves as a topic.
        // This is static and can be calculated at compile time.
        if !event.anonymous {
            let mut encoded =
                format!("{}::{}", &self.ns.contracts[contract_no].name, &event.name).encode();
            encoded[0] = 0; // Set the prefix
            let len_encoded = encoded.len();
            let mut result = Hash::clear();
            let len_result = result.as_ref().len();
            let result = if len_encoded <= len_result {
                result.as_mut()[..len_encoded].copy_from_slice(&encoded);
                result
            } else {
                let mut hash_output = <<Blake2x256 as HashOutput>::Type as Default>::default();
                <Blake2x256 as CryptoHash>::hash(&encoded, &mut hash_output);
                let copy_len = core::cmp::min(hash_output.len(), len_result);
                result.as_mut()[0..copy_len].copy_from_slice(&hash_output[0..copy_len]);
                result
            };
            let result = result.as_ref().to_vec();
            topics.push(Expression::AllocDynamicArray(
                loc,
                Type::Slice(Type::Uint(8).into()),
                Expression::NumberLiteral(loc, Type::Uint(32), result.len().into()).into(),
                Some(result),
            ));
            topic_tys.push(topic_ty());
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

        for (i, arg) in self.args.iter().enumerate() {
            if self.ns.events[self.event_no].fields[i].indexed {
                //let ty = arg.ty();
                // TODO if the topic prefix is ALREADY 32 bytes or more we can spare us a branch

                let e = expression(arg, cfg, contract_no, Some(func), self.ns, vartab, opt);

                let e = Expression::AbiEncode {
                    loc,
                    tys: vec![e.ty()],
                    packed: vec![],
                    args: vec![e],
                };

                let concatenated = Expression::StringConcat(
                    loc,
                    Type::DynamicBytes,
                    ast::StringLocation::CompileTime(topic_prefixes.remove(0)), // TODO not efficient
                    ast::StringLocation::RunTime(e.into()),
                );
                assert_eq!(concatenated.ty(), Type::DynamicBytes);

                vartab.new_dirty_tracker();

                let var = vartab.temp_anonymous(&Type::DynamicBytes);

                cfg.add(
                    vartab,
                    Instr::Set {
                        loc,
                        res: var,
                        expr: concatenated,
                    },
                );

                let unhashed = Expression::Variable(loc, topic_ty(), var);

                let compare = Expression::UnsignedMore(
                    loc,
                    Expression::Builtin(
                        loc,
                        vec![Type::Uint(32)],
                        Builtin::ArrayLength,
                        vec![unhashed.clone()],
                    )
                    .into(),
                    Expression::NumberLiteral(loc, Type::Uint(32), 32.into()).into(),
                );

                let bigger = cfg.new_basic_block("bigger".into());
                //let smaller = cfg.new_basic_block("smaller".into());
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
                    Instr::Set {
                        loc,
                        res: var,
                        expr: Expression::Builtin(
                            loc,
                            vec![Type::Uint(32)],
                            Builtin::Blake2_256,
                            vec![unhashed.clone()],
                        ),
                    },
                );
                vartab.set_dirty(var);
                cfg.add(vartab, Instr::Branch { block: done });

                //cfg.set_basic_block(smaller);

                //vartab.set_dirty(var);

                //cfg.add(vartab, Instr::Branch { block: done });

                cfg.set_basic_block(done);

                cfg.set_phis(done, vartab.pop_dirty_tracker());

                topics.push(unhashed);
                topic_tys.push(topic_ty());
            } else {
                let e = expression(arg, cfg, contract_no, Some(func), self.ns, vartab, opt);

                data.push(e);
                data_tys.push(arg.ty());
            }
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
