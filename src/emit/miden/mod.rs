// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::Instr;
use crate::codegen::Expression;
use crate::codegen::{cfg::ControlFlowGraph, Options};

use crate::{emit::Binary, sema::ast};
use inkwell::{context::Context, module::Module};
pub(super) mod target;

pub struct Midentarget;

impl Midentarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        opt: &'a Options,
        _contract_no: usize,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut bin = Binary::new(
            context,
            ns,
            &contract.id.name,
            &filename,
            opt,
            std_lib,
            None,
        );

        // for each cfg, wrap its instructions in a function and copy it to the binary
        for cfg in &contract.cfg {
            if cfg.name == "storage_initializer" {
                Self::emit_storage_initializer(cfg, &mut bin);
                continue;
            }

            if cfg.name.contains("constructor") {
                continue;
            }

            let name = cfg.name.split("::").last().unwrap();
            bin.miden_instrs
                .as_ref()
                .unwrap()
                .borrow_mut()
                .push(format!("export.{}", name));

            for miden_instr in cfg.miden_instrs.iter() {
                bin.miden_instrs
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .push(miden_instr.to_string());
            }
            bin.miden_instrs
                .as_ref()
                .unwrap()
                .borrow_mut()
                .push("end".to_string());
        }

        bin
    }

    pub fn emit_storage_initializer(cfg: &ControlFlowGraph, bin: &mut Binary) {
        let use_miden_account = "use.miden::account";
        let use_miden_sys = "use.std::sys";

        bin.miden_instrs
            .as_ref()
            .unwrap()
            .borrow_mut()
            .insert(0, use_miden_account.to_string());
        bin.miden_instrs
            .as_ref()
            .unwrap()
            .borrow_mut()
            .insert(1, use_miden_sys.to_string());

        for instr in cfg.blocks[0].instr.iter().enumerate() {
            if let Instr::SetStorage {
                ty: _ty,
                value: _value,
                storage,
                storage_type: _storage_type,
            } = instr.1
            {
                if let Expression::NumberLiteral {
                    loc: _loc,
                    ty: _ty,
                    value,
                } = storage
                {
                    println!("Set storage at slot: {}", value);

                    let miden_instr = format!("const.STORAGE_SLOT_{}={}", value, value);
                    bin.miden_instrs
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .insert(instr.0 + 2, miden_instr);
                }
            }
        }
    }
}
