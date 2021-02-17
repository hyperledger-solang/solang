use super::Contract;
use inkwell::basic_block::BasicBlock;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PhiValue};
use inkwell::IntPredicate;
use std::collections::HashMap;

pub struct LoopBuilder<'a> {
    phis: HashMap<&'static str, PhiValue<'a>>,
    entry_block: BasicBlock<'a>,
    condition_block: BasicBlock<'a>,
    body_block: BasicBlock<'a>,
    done_block: BasicBlock<'a>,
}

impl<'a> LoopBuilder<'a> {
    pub fn new(contract: &'a Contract, function: FunctionValue) -> Self {
        let entry_block = contract.builder.get_insert_block().unwrap();
        let condition_block = contract.context.append_basic_block(function, "cond");
        let body_block = contract.context.append_basic_block(function, "body");
        let done_block = contract.context.append_basic_block(function, "done");

        contract.builder.build_unconditional_branch(condition_block);

        contract.builder.position_at_end(condition_block);

        LoopBuilder {
            phis: HashMap::new(),
            entry_block,
            condition_block,
            body_block,
            done_block,
        }
    }

    pub fn add_loop_phi<T: BasicType<'a>>(
        &mut self,
        contract: &'a Contract,
        name: &'static str,
        ty: T,
        initial_value: &BasicValueEnum<'a>,
    ) {
        let phi = contract.builder.build_phi(ty, name);

        phi.add_incoming(&[(initial_value, self.entry_block)]);

        self.phis.insert(name, phi);
    }

    pub fn over(&self, contract: &'a Contract, from: IntValue<'a>, to: IntValue<'a>) {
        let loop_ty = from.get_type();
        let loop_phi = contract.builder.build_phi(loop_ty, "index");

        let loop_var = loop_phi.as_basic_value().into_int_value();

        let next =
            contract
                .builder
                .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = contract
            .builder
            .build_int_compare(IntPredicate::ULT, loop_var, to, "loop_cond");

        contract
            .builder
            .build_conditional_branch(comp, self.body_block, self.done_block);

        loop_phi.add_incoming(&[(&from, self.entry_block), (&next, self.body_block)]);

        contract.builder.position_at_end(self.body_block);
    }

    pub fn get_loop_phi(&self, name: &'static str) -> BasicValueEnum {
        self.phis[name].as_basic_value()
    }

    pub fn set_loop_phi(
        &self,
        contract: &'a Contract,
        name: &'static str,
        value: &BasicValueEnum<'a>,
    ) {
        let block = contract.builder.get_insert_block().unwrap();

        self.phis[name].add_incoming(&[(value, block)]);
    }

    pub fn finish(&self, contract: &'a Contract) {
        contract
            .builder
            .build_unconditional_branch(self.condition_block);

        contract.builder.position_at_end(self.done_block);
    }
}
