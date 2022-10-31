// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::Options;
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use solang_parser::pt;

/// This struct implements the trait 'EventEmitter' in order to handle the emission of events
/// for Substrate
pub(super) struct SubstrateEventEmitter<'a> {
    /// Arguments passed to the event
    pub(super) args: &'a [ast::Expression],
    pub(super) ns: &'a Namespace,
    pub(super) event_no: usize,
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
