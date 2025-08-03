// SPDX-License-Identifier: Apache-2.0

use crate::codegen::{cfg::ControlFlowGraph, vartable::Storage};
use crate::emit::binary::Binary;
use crate::emit::instructions::process_instruction;
use crate::emit::{TargetRuntime, Variable};
use crate::sema::ast::Contract;
use crate::Target;
use inkwell::debug_info::{AsDIScope, DISubprogram, DIType};
use inkwell::types::BasicType;
use inkwell::values::{FunctionValue, PhiValue};
use inkwell::AddressSpace;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;
use std::collections::{HashMap, VecDeque};

// recurse through basic blocks
pub(super) struct BasicBlock<'a> {
    pub(super) bb: inkwell::basic_block::BasicBlock<'a>,
    pub(super) phis: HashMap<usize, PhiValue<'a>>,
}

pub(super) struct Work<'b> {
    pub(super) block_no: usize,
    pub(super) vars: HashMap<usize, Variable<'b>>,
}

pub(super) fn emit_cfg<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &mut T,
    bin: &mut Binary<'a>,
    contract: &Contract,
    cfg: &ControlFlowGraph,
    function: FunctionValue<'a>,
) {
    let dibuilder = &bin.dibuilder;
    let compile_unit = &bin.compile_unit;
    let file = compile_unit.get_file();
    let mut di_func_scope: Option<DISubprogram<'_>> = None;

    if bin.options.generate_debug_information {
        let return_type = function.get_type().get_return_type();
        match return_type {
            None => {}
            Some(return_type) => {
                let return_type_size = return_type.size_of().unwrap();
                let size = return_type_size.get_type().get_bit_width();
                let mut type_name = "size_".to_owned();
                type_name.push_str(&size.to_string());
                let di_flags = if cfg.public {
                    inkwell::debug_info::DIFlagsConstants::PUBLIC
                } else {
                    inkwell::debug_info::DIFlagsConstants::PRIVATE
                };

                let di_return_type = dibuilder
                    .create_basic_type(&type_name, size as u64, 0x00, di_flags)
                    .unwrap();
                let di_param_types: Vec<DIType<'_>> = cfg
                    .params
                    .iter()
                    .map(|param| {
                        let name = param.ty.to_string(bin.ns);
                        dibuilder
                            .create_basic_type(&name, param.ty.bits(bin.ns).into(), 0x00, di_flags)
                            .unwrap()
                            .as_type()
                    })
                    .collect();
                let di_func_type = dibuilder.create_subroutine_type(
                    file,
                    Some(di_return_type.as_type()),
                    di_param_types.as_slice(),
                    di_flags,
                );

                let func_loc = cfg.blocks[0].instr.first().unwrap().loc();
                let line_num = if let pt::Loc::File(file_offset, offset, _) = func_loc {
                    let (line, _) = bin.ns.files[file_offset].offset_to_line_column(offset);
                    line
                } else {
                    0
                };

                di_func_scope = Some(dibuilder.create_function(
                    compile_unit.as_debug_info_scope(),
                    function.get_name().to_str().unwrap(),
                    None,
                    file,
                    line_num.try_into().unwrap(),
                    di_func_type,
                    true,
                    true,
                    line_num.try_into().unwrap(),
                    di_flags,
                    false,
                ));
                function.set_subprogram(di_func_scope.unwrap());
            }
        }
    }

    let mut blocks: HashMap<usize, BasicBlock> = HashMap::new();

    let mut work = VecDeque::new();

    blocks.insert(0, create_block(0, bin, cfg, function));

    // On Solana, the last argument is the accounts
    if bin.ns.target == Target::Solana {
        bin.parameters = Some(function.get_last_param().unwrap().into_pointer_value());
    }

    // Create all the stack variables
    let mut vars = HashMap::new();

    for (no, v) in &cfg.vars {
        match v.storage {
            Storage::Local if v.ty.is_reference_type(bin.ns) && !v.ty.is_contract_storage() => {
                // a null pointer means an empty, zero'ed thing, be it string, struct or array
                let value = bin
                    .context
                    .ptr_type(AddressSpace::default())
                    .const_null()
                    .into();

                vars.insert(*no, Variable { value });
            }
            Storage::Local if v.ty.is_contract_storage() => {
                vars.insert(
                    *no,
                    Variable {
                        value: bin
                            .llvm_type(&bin.ns.storage_type())
                            .into_int_type()
                            .const_zero()
                            .into(),
                    },
                );
            }
            Storage::Constant(_) | Storage::Contract(_) if v.ty.is_reference_type(bin.ns) => {
                // This needs a placeholder
                vars.insert(
                    *no,
                    Variable {
                        value: bin.context.bool_type().get_undef().into(),
                    },
                );
            }
            Storage::Local | Storage::Contract(_) | Storage::Constant(_) => {
                let ty = bin.llvm_type(&v.ty);
                vars.insert(
                    *no,
                    Variable {
                        value: if ty.is_pointer_type() {
                            ty.into_pointer_type().const_zero().into()
                        } else if ty.is_array_type() {
                            ty.into_array_type().const_zero().into()
                        } else if ty.is_int_type() {
                            ty.into_int_type().const_zero().into()
                        } else {
                            ty.into_struct_type().const_zero().into()
                        },
                    },
                );
            }
        }
    }

    work.push_back(Work { block_no: 0, vars });

    while let Some(mut w) = work.pop_front() {
        let bb = blocks.get(&w.block_no).unwrap();

        bin.builder.position_at_end(bb.bb);

        for (v, phi) in bb.phis.iter() {
            w.vars.get_mut(v).unwrap().value = (*phi).as_basic_value();
        }

        for ins in &cfg.blocks[w.block_no].instr {
            if bin.options.generate_debug_information {
                let debug_loc = ins.loc();
                if let pt::Loc::File(file_offset, offset, _) = debug_loc {
                    let (line, col) = bin.ns.files[file_offset].offset_to_line_column(offset);
                    let debug_loc = dibuilder.create_debug_location(
                        bin.context,
                        line as u32,
                        col as u32,
                        di_func_scope.unwrap().as_debug_info_scope(),
                        None,
                    );
                    bin.builder.set_current_debug_location(debug_loc);
                } else {
                    // For instructions that do not have a location, insert a debug location pointing to line 0.
                    // If -g flag is enabled, every instruction should have a debug location. This is necessary
                    // because llvm's inliner pass requires function call instructions to have a debug location.
                    let debug_loc = dibuilder.create_debug_location(
                        bin.context,
                        0_u32,
                        0_u32,
                        di_func_scope.unwrap().as_debug_info_scope(),
                        None,
                    );
                    bin.builder.set_current_debug_location(debug_loc);
                }
            }

            process_instruction(
                target,
                ins,
                bin,
                &mut w,
                function,
                cfg,
                &mut work,
                &mut blocks,
                contract,
            );
            bin.builder.unset_current_debug_location();
            dibuilder.finalize();
        }
    }
}

pub(super) fn create_block<'a>(
    block_no: usize,
    bin: &Binary<'a>,
    cfg: &ControlFlowGraph,
    function: FunctionValue<'a>,
) -> BasicBlock<'a> {
    let cfg_bb = &cfg.blocks[block_no];
    let mut phis = HashMap::new();

    let bb = bin.context.append_basic_block(function, &cfg_bb.name);

    bin.builder.position_at_end(bb);

    if let Some(ref cfg_phis) = cfg_bb.phis {
        for v in cfg_phis {
            let ty = bin.llvm_var_ty(&cfg.vars[v].ty);

            phis.insert(*v, bin.builder.build_phi(ty, &cfg.vars[v].id.name).unwrap());
        }
    }

    BasicBlock { bb, phis }
}
