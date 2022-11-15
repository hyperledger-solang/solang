// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use ink_env::hash::{Blake2x256, CryptoHash, HashOutput};
use ink_env::topics::PrefixedValue;
use ink_primitives::{Clear, Hash};
use parity_scale_codec as scale;
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

        let event = &self.ns.events[self.event_no];

        // Events that are not anonymous always have themselves as a topic.
        // This is static and can be calculated at compile time.
        let mut topic_hashes = Vec::new();
        if !event.anonymous {
            let value = format!("{}::{}", &self.ns.contracts[contract_no].name, &event.name);
            topic_hashes.push(encoded_into_hash(&PrefixedValue {
                prefix: b"",
                value: &value.as_bytes(),
            }));
        };

        // Topic prefixes are static and can be calculated at compile time.
        let topic_prefixes: Vec<Vec<u8>> = event
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
            })
            .collect();

        for (i, arg) in self.args.iter().enumerate() {
            if self.ns.events[self.event_no].fields[i].indexed {
                let ty = arg.ty();

                match ty {
                    Type::String | Type::DynamicBytes => {
                        let e = expression(
                            &ast::Expression::Builtin(
                                pt::Loc::Codegen,
                                vec![Type::Bytes(32)],
                                ast::Builtin::Keccak256,
                                vec![arg.clone()],
                            ),
                            cfg,
                            contract_no,
                            Some(func),
                            self.ns,
                            vartab,
                            opt,
                        );

                        topics.push(e);
                        topic_tys.push(Type::Bytes(32));
                    }
                    Type::Struct(_) | Type::Array(..) => {
                        // We should have an AbiEncodePackedPad
                        let e = expression(
                            &ast::Expression::Builtin(
                                pt::Loc::Codegen,
                                vec![Type::Bytes(32)],
                                ast::Builtin::Keccak256,
                                vec![ast::Expression::Builtin(
                                    pt::Loc::Codegen,
                                    vec![Type::DynamicBytes],
                                    ast::Builtin::AbiEncodePacked,
                                    vec![arg.clone()],
                                )],
                            ),
                            cfg,
                            contract_no,
                            Some(func),
                            self.ns,
                            vartab,
                            opt,
                        );

                        topics.push(e);
                        topic_tys.push(Type::Bytes(32));
                    }
                    _ => {
                        let e = expression(arg, cfg, contract_no, Some(func), self.ns, vartab, opt);

                        topics.push(e);
                        topic_tys.push(ty);
                    }
                }
            } else {
                // TODO: In ink all topics are stuffed into an Enum
                // The enum variant number will be in the encoded event
                // So we need to make sure this matches the order within the metadata?

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
