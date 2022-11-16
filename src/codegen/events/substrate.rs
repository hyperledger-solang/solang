// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Expression, Instr};
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

                let e = expression(arg, cfg, contract_no, Some(func), self.ns, vartab, opt);

                let e = Expression::AbiEncode {
                    loc: pt::Loc::Codegen,
                    tys: vec![Type::String, e.ty()],
                    packed: vec![],
                    args: vec![e],
                };

                let prefix = Expression::AllocDynamicArray(
                    Loc::Codegen,
                    Type::Slice,
                    Some(b"Foo:bar:bar".into()),
                );

                let concatenated = Expression::StringConcat(prefix, e);

                assert_eq!(concatenated.ty(), Type::DynamicBytes);

                let compare = Expression::More(
                    Loc::Codegen,
                    Expression::Builtin(Loc::Codegen, Builtin::ArrayLength, vec![concatenated]),
                    Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 32.into()),
                );

                let bigger = cfg.new_basic_block("bigger");
                let smaller = cfg.new_basic_block("smaller");
                let done = cfg.new_basic_block("done");

                vartab.new_dirty_tracker();

                let var = vartab.temp_anonymous(&Type::DynamicBytes);

                cfg.add(vartab, Instr::BranchCond {cond: compare, true_block: bigger, false_block: smaller});

                cfg.set_basic_block(bigger);

                cfg.add(Instr::Set { res: var, expr: Expression::Builtin(Loc::Codegen, Builtin::Blake2_256, vec![concatenated])});

                vartab.set_dirty(var);

                cfg.add(vartab, Instr::Branch { block: done });

                cfg.set_basic_block(smaller);

                cfg.add(Instr::Set { res: var, expr: concatenated)});
                vartab.set_dirty(var);
                cfg.add(vartab, Instr::Branch { block: done });

                cfg.set_phis(done, vartab.pop_dirty_tracker());

                topics.push(Expression::Variable(Loc::Codegen, Type::DynamicBytes, var));
                topic_tys.push(ty);
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
