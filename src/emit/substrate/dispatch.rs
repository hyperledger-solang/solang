// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, ReturnCode};
use crate::emit::binary::Binary;
use crate::emit::functions::abort_if_value_transfer;
use crate::emit::substrate::SubstrateTarget;
use crate::emit::TargetRuntime;
use crate::sema::ast::{Contract, Namespace, Type};
use crate::Target;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, FunctionValue};
use inkwell::{AddressSpace, IntPredicate};
use solang_parser::pt;
use std::collections::HashMap;

impl SubstrateTarget {
    /// Create function dispatch based on abi encoded argsdata. The dispatcher loads the leading function selector,
    /// and dispatches based on that. If no function matches this, or no selector is in the argsdata, then fallback
    /// code is executed. This is either a fallback block provided to this function, or it automatically dispatches
    /// to the fallback function or receive function, if any.
    pub(super) fn emit_function_dispatch<'a, F>(
        &self,
        bin: &Binary<'a>,
        contract: &Contract,
        ns: &Namespace,
        function_ty: pt::FunctionTy,
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        functions: &HashMap<usize, FunctionValue<'a>>,
        fallback: Option<inkwell::basic_block::BasicBlock>,
        nonpayable: F,
    ) where
        F: Fn(&ControlFlowGraph) -> bool,
    {
        // create start function
        let no_function_matched = match fallback {
            Some(block) => block,
            None => bin
                .context
                .append_basic_block(function, "no_function_matched"),
        };

        let switch_block = bin.context.append_basic_block(function, "switch");

        let not_fallback = bin.builder.build_int_compare(
            IntPredicate::UGE,
            argslen,
            argslen.get_type().const_int(4, false),
            "",
        );

        bin.builder
            .build_conditional_branch(not_fallback, switch_block, no_function_matched);

        bin.builder.position_at_end(switch_block);

        let fid = bin
            .builder
            .build_load(bin.context.i32_type(), argsdata, "function_selector")
            .into_int_value();

        // TODO: solana does not support bss, so different solution is needed
        bin.builder
            .build_store(bin.selector.as_pointer_value(), fid);

        // step over the function selector
        let argsdata = unsafe {
            bin.builder.build_gep(
                bin.context.i32_type(),
                argsdata,
                &[bin.context.i32_type().const_int(1, false)],
                "argsdata",
            )
        };

        let argslen =
            bin.builder
                .build_int_sub(argslen, argslen.get_type().const_int(4, false), "argslen");

        let mut cases = Vec::new();

        for (cfg_no, cfg) in contract.cfg.iter().enumerate() {
            if cfg.ty != function_ty || !cfg.public {
                continue;
            }

            self.add_dispatch_case(
                bin,
                cfg,
                ns,
                &mut cases,
                argsdata,
                argslen,
                function,
                functions[&cfg_no],
                &nonpayable,
            );
        }

        bin.builder.position_at_end(switch_block);

        bin.builder.build_switch(fid, no_function_matched, &cases);

        if fallback.is_some() {
            return; // caller will generate fallback code
        }

        // emit fallback code
        bin.builder.position_at_end(no_function_matched);

        let fallback = contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Fallback);

        let receive = contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Receive);

        if fallback.is_none() && receive.is_none() {
            // no need to check value transferred; we will abort either way
            self.return_code(bin, bin.return_values[&ReturnCode::FunctionSelectorInvalid]);

            return;
        }

        let got_value = if bin.function_abort_value_transfers {
            bin.context.bool_type().const_zero()
        } else {
            let value = self.value_transferred(bin, ns);

            bin.builder.build_int_compare(
                IntPredicate::NE,
                value,
                bin.value_type(ns).const_zero(),
                "is_value_transfer",
            )
        };

        let fallback_block = bin.context.append_basic_block(function, "fallback");
        let receive_block = bin.context.append_basic_block(function, "receive");

        bin.builder
            .build_conditional_branch(got_value, receive_block, fallback_block);

        bin.builder.position_at_end(fallback_block);

        match fallback {
            Some((cfg_no, _)) => {
                let args = vec![];
                bin.builder.build_call(functions[&cfg_no], &args, "");
                self.return_empty_abi(bin);
            }
            None => {
                self.return_code(bin, bin.context.i32_type().const_int(2, false));
            }
        }

        bin.builder.position_at_end(receive_block);

        match receive {
            Some((cfg_no, _)) => {
                let args = if ns.target == Target::Solana {
                    vec![function.get_last_param().unwrap().into()]
                } else {
                    vec![]
                };

                bin.builder.build_call(functions[&cfg_no], &args, "");

                self.return_empty_abi(bin);
            }
            None => {
                self.return_code(bin, bin.context.i32_type().const_int(2, false));
            }
        }
    }

    /// Add single case for emit_function_dispatch
    fn add_dispatch_case<'a, F>(
        &self,
        bin: &Binary<'a>,
        f: &ControlFlowGraph,
        ns: &Namespace,
        cases: &mut Vec<(
            inkwell::values::IntValue<'a>,
            inkwell::basic_block::BasicBlock<'a>,
        )>,
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        dest: inkwell::values::FunctionValue<'a>,
        nonpayable: &F,
    ) where
        F: Fn(&ControlFlowGraph) -> bool,
    {
        let bb = bin.context.append_basic_block(function, "");

        bin.builder.position_at_end(bb);

        if nonpayable(f) {
            abort_if_value_transfer(self, bin, function, ns);
        }

        let mut args = Vec::new();

        // insert abi decode
        self.abi_decode(bin, function, &mut args, argsdata, argslen, &f.params, ns);

        // add return values as pointer arguments at the end
        if !f.returns.is_empty() {
            for v in f.returns.iter() {
                args.push(if !v.ty.is_reference_type(ns) {
                    bin.build_alloca(function, bin.llvm_type(&v.ty, ns), v.name_as_str())
                        .into()
                } else {
                    bin.build_alloca(
                        function,
                        bin.llvm_type(&v.ty, ns).ptr_type(AddressSpace::default()),
                        v.name_as_str(),
                    )
                    .into()
                });
            }
        }

        let meta_args: Vec<BasicMetadataValueEnum> = args.iter().map(|arg| (*arg).into()).collect();

        let ret = bin
            .builder
            .build_call(dest, &meta_args, "")
            .try_as_basic_value()
            .left()
            .unwrap();

        let success = bin.builder.build_int_compare(
            IntPredicate::EQ,
            ret.into_int_value(),
            bin.return_values[&ReturnCode::Success],
            "success",
        );

        let success_block = bin.context.append_basic_block(function, "success");
        let bail_block = bin.context.append_basic_block(function, "bail");

        bin.builder
            .build_conditional_branch(success, success_block, bail_block);

        bin.builder.position_at_end(success_block);

        if f.returns.is_empty() {
            // return ABI of length 0
            self.return_empty_abi(bin);
        } else {
            let tys: Vec<Type> = f.returns.iter().map(|p| p.ty.clone()).collect();

            let (data, length) = self.abi_encode(
                bin,
                None,
                true,
                function,
                &args[f.params.len()..f.params.len() + f.returns.len()],
                &tys,
                ns,
            );

            self.return_abi(bin, data, length);
        }

        bin.builder.position_at_end(bail_block);

        self.return_code(bin, ret.into_int_value());

        cases.push((
            bin.context.i32_type().const_int(
                u32::from_le_bytes(f.selector.as_slice().try_into().unwrap()) as u64,
                false,
            ),
            bb,
        ));
    }
}
