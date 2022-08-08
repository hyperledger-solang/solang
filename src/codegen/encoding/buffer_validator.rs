use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::vartable::Vartable;
use crate::codegen::Expression;
use crate::sema::ast::{Namespace, Type};
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::pt::Loc;
use std::ops::AddAssign;

pub(super) struct BufferValidator<'a> {
    pub(super) buffer_length: Expression,
    pub(super) types: &'a [Type],
    pub(super) verified_until: i32,
    pub(super) current_arg: usize,
}

impl BufferValidator<'_> {
    pub(super) fn set_argument_number(&mut self, arg_no: usize) {
        self.current_arg = arg_no;
    }

    pub(super) fn initialize_validation(
        &mut self,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        self.verified_until = -1;
        self._verify_buffer(offset, ns, vartab, cfg);
    }

    pub(super) fn validate_buffer(
        &mut self,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        if self.current_arg as i32 <= self.verified_until {
            return;
        }

        self._verify_buffer(offset, ns, vartab, cfg);
    }

    pub(super) fn validate_offset(
        &self,
        offset: Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        self.build_branch(offset, vartab, cfg);
    }

    pub(super) fn validation_necessary(&self) -> bool {
        self.current_arg as i32 > self.verified_until
    }

    pub(super) fn validate_array_offset(
        &mut self,
        offset: Expression,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        self.build_branch(offset, vartab, cfg);
        self.verified_until = self.current_arg as i32;
    }

    fn _verify_buffer(
        &mut self,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        let mut maximum_verifiable = self.current_arg;
        for i in self.current_arg..self.types.len() {
            if !self.types[i].is_dynamic(ns) {
                maximum_verifiable = i;
            } else {
                break;
            }
        }

        if maximum_verifiable == self.current_arg {
            return;
        }

        let mut advance = BigInt::zero();
        for i in self.current_arg..=maximum_verifiable {
            advance.add_assign(self.types[i].memory_size_of(ns));
        }

        let reach = Expression::Add(
            Loc::Codegen,
            Type::Uint(32),
            false,
            Box::new(offset.clone()),
            Box::new(Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(32),
                advance,
            )),
        );

        self.verified_until = maximum_verifiable as i32;
        self.build_branch(reach, vartab, cfg);
    }

    fn build_branch(&self, offset: Expression, vartab: &mut Vartable, cfg: &mut ControlFlowGraph) {
        let cond = Expression::LessEqual(
            Loc::Codegen,
            Box::new(offset),
            Box::new(self.buffer_length.clone()),
        );

        let inbounds_block = cfg.new_basic_block("inbounds".to_string());
        let out_of_bounds_block = cfg.new_basic_block("out_of_bounds".to_string());

        cfg.add(
            vartab,
            Instr::BranchCond {
                cond,
                true_block: inbounds_block,
                false_block: out_of_bounds_block,
            },
        );

        cfg.set_basic_block(out_of_bounds_block);
        // TODO: Add an error message here
        cfg.add(vartab, Instr::AssertFailure { expr: None });
        cfg.set_basic_block(inbounds_block);
    }
}
