// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::encoding::soroban_encoding::soroban_encode_arg;
use crate::codegen::events::EventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast::{self, Function, Namespace, Type};
use num_bigint::BigInt;
use solang_parser::pt;

/// Implements [EventEmitter] for the Soroban target.
///
/// Each indexed Solidity event field becomes a topic Val in the Soroban
/// `contract_event` call. The first non-indexed field becomes the data Val.
/// If there are no non-indexed fields, a zero Val is passed as data.
pub(super) struct SorobanEventEmitter<'a> {
    pub(super) loc: pt::Loc,
    pub(super) args: &'a [ast::Expression],
    pub(super) ns: &'a Namespace,
    pub(super) event_no: usize,
}

impl EventEmitter for SorobanEventEmitter<'_> {
    fn selector(&self, _: usize) -> Vec<u8> {
        // Soroban events are identified by topics, not a selector discriminator.
        vec![]
    }

    fn emit(
        &self,
        contract_no: usize,
        func: &Function,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        opt: &Options,
    ) {
        let event = &self.ns.events[self.event_no];
        let mut topics: Vec<Expression> = Vec::new();
        let mut data_args: Vec<Expression> = Vec::new();

        for (ast_exp, field) in self.args.iter().zip(event.fields.iter()) {
            let value = expression(ast_exp, cfg, contract_no, Some(func), self.ns, vartab, opt);
            let encoded = soroban_encode_arg(value, cfg, vartab, self.ns);
            if field.indexed {
                topics.push(encoded);
            } else {
                data_args.push(encoded);
            }
        }

        // Soroban's contract_event takes a single Val for data. Use the first
        // non-indexed arg if one exists; otherwise pass a zero Val.
        let data = data_args
            .into_iter()
            .next()
            .unwrap_or_else(|| Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(64),
                value: BigInt::from(0u64),
            });

        cfg.add(
            vartab,
            Instr::EmitEvent {
                event_no: self.event_no,
                data,
                topics,
            },
        );
    }
}
