use super::cfg::{BasicBlock, ControlFlowGraph, Instr};
use super::reaching_definitions::{Def, Transfer};
use crate::sema::ast::{Expression, Namespace, Type};
use indexmap::IndexMap;
use std::collections::HashSet;

/// A vector is a modifiable struct with a length, size and data. A slice is a readonly
/// pointer to some data, plus the length. By using a slice, often a memcpy can be avoided.
///
/// Codegen generates vectors. Here we walk the cfg to find all vectors which can be converted
/// to slices. In addition, we add some notes to the namespace so the language server can display
/// some information when hovering over a variable.
pub fn vector_to_slice(cfg: &mut ControlFlowGraph, ns: &mut Namespace) {
    // first, we need to find all the defs which have modified their referent
    // note that variables can aliases
    let mut writable = HashSet::new();

    for block_no in 0..cfg.blocks.len() {
        let mut vars = cfg.blocks[block_no].defs.clone();

        find_writable_vectors(&cfg.blocks[block_no], &mut vars, &mut writable);
    }

    // Now we have a list of all vectors defs that get written two (via variables)

    // walk the cfg and expressions and update the type of vectors
    update_vectors_to_slice(&writable, cfg, ns);
}

fn find_writable_vectors(
    block: &BasicBlock,
    vars: &mut IndexMap<usize, IndexMap<Def, bool>>,
    writable: &mut HashSet<Def>,
) {
    for instr_no in 0..block.instr.len() {
        match &block.instr[instr_no] {
            Instr::Set {
                res,
                expr: Expression::Variable(_, _, var_no),
                ..
            } => {
                // is this aliasing a vector var
                if let Some(defs) = vars.get(var_no) {
                    let defs = defs.clone();

                    apply_transfers(&block.transfers[instr_no], vars, writable);

                    vars.insert(*res, defs);
                } else {
                    apply_transfers(&block.transfers[instr_no], vars, writable);
                }
            }
            // Call and return do not take slices
            Instr::Return { value: args } | Instr::Call { args, .. } => {
                for arg in args {
                    if let Expression::Variable(_, _, var_no) = arg {
                        if let Some(entry) = vars.get_mut(var_no) {
                            writable.extend(entry.keys());
                        }
                    }
                }

                apply_transfers(&block.transfers[instr_no], vars, writable);
            }
            Instr::PushMemory { value, .. } => {
                if let Expression::Variable(_, _, var_no) = value.as_ref() {
                    if let Some(entry) = vars.get_mut(var_no) {
                        writable.extend(entry.keys());
                    }
                }

                apply_transfers(&block.transfers[instr_no], vars, writable);
            }
            Instr::Store { pos, .. } => {
                if let Some(entry) = vars.get_mut(pos) {
                    writable.extend(entry.keys());
                }

                apply_transfers(&block.transfers[instr_no], vars, writable);
            }
            Instr::WriteBuffer { buf, .. } => {
                if let Expression::Variable(_, _, var_no) = buf {
                    if let Some(entry) = vars.get_mut(var_no) {
                        writable.extend(entry.keys());
                    }
                }

                apply_transfers(&block.transfers[instr_no], vars, writable);
            }
            // These instructions are fine with vectors
            Instr::Set { .. }
            | Instr::Nop
            | Instr::Branch { .. }
            | Instr::BranchCond { .. }
            | Instr::PopMemory { .. }
            | Instr::LoadStorage { .. }
            | Instr::SetStorage { .. }
            | Instr::ClearStorage { .. }
            | Instr::SetStorageBytes { .. }
            | Instr::PushStorage { .. }
            | Instr::PopStorage { .. }
            | Instr::SelfDestruct { .. }
            | Instr::EmitEvent { .. }
            | Instr::AbiDecode { .. }
            | Instr::ExternalCall { .. }
            | Instr::Constructor { .. }
            | Instr::Unreachable
            | Instr::Print { .. }
            | Instr::AssertFailure { .. }
            | Instr::ValueTransfer { .. } => {
                apply_transfers(&block.transfers[instr_no], vars, writable);
            }
        }
    }
}

fn apply_transfers(
    transfers: &[Transfer],
    vars: &mut IndexMap<usize, IndexMap<Def, bool>>,
    writable: &mut HashSet<Def>,
) {
    for transfer in transfers {
        match transfer {
            Transfer::Kill { var_no } => {
                vars.remove(var_no);
            }
            Transfer::Mod { var_no } => {
                if let Some(entry) = vars.get_mut(var_no) {
                    for e in entry.values_mut() {
                        *e = true;
                    }

                    writable.extend(entry.keys());
                }
            }
            Transfer::Copy { var_no, src } => {
                if let Some(defs) = vars.get(src) {
                    let defs = defs.clone();

                    vars.insert(*var_no, defs);
                }
            }
            Transfer::Gen { var_no, def } => {
                if let Some(entry) = vars.get_mut(var_no) {
                    entry.insert(*def, false);
                } else {
                    let mut v = IndexMap::new();
                    v.insert(*def, false);
                    vars.insert(*var_no, v);
                }
            }
        }
    }
}

fn update_vectors_to_slice(
    writable: &HashSet<Def>,
    cfg: &mut ControlFlowGraph,
    ns: &mut Namespace,
) {
    let mut defs_to_be_updated: HashSet<Def> = HashSet::new();

    for block_no in 0..cfg.blocks.len() {
        for instr_no in 0..cfg.blocks[block_no].instr.len() {
            let cur = Def { block_no, instr_no };

            if let Instr::Set {
                expr: Expression::AllocDynamicArray(..),
                ..
            } = &cfg.blocks[block_no].instr[instr_no]
            {
                if !writable.contains(&cur) {
                    defs_to_be_updated.insert(cur);
                }
            }
        }
    }

    for block_no in 0..cfg.blocks.len() {
        if let Some(phis) = &cfg.blocks[block_no].phis {
            for phi in phis {
                // if any of the defs is not going to be updated ...

                // note that unreachable blocks do not have reaching defs calculated
                if cfg.blocks[block_no].defs.contains_key(phi)
                    && cfg.blocks[block_no].defs[phi]
                        .iter()
                        .any(|(def, _)| !defs_to_be_updated.contains(def))
                {
                    // don't update any of them
                    for (def, _) in &cfg.blocks[block_no].defs[phi] {
                        defs_to_be_updated.remove(def);
                    }
                }
            }
        }
    }

    for def in defs_to_be_updated {
        if let Instr::Set {
            loc,
            res,
            expr: Expression::AllocDynamicArray(_, _, len, Some(bs)),
        } = &cfg.blocks[def.block_no].instr[def.instr_no]
        {
            let res = *res;
            cfg.blocks[def.block_no].instr[def.instr_no] = Instr::Set {
                loc: *loc,
                res,
                expr: Expression::AllocDynamicArray(
                    *loc,
                    Type::Slice,
                    len.clone(),
                    Some(bs.clone()),
                ),
            };

            if let Some(function_no) = cfg.function_no {
                if let Some(var) = ns.functions[function_no].symtable.vars.get_mut(&res) {
                    var.slice = true;
                }
            }
        }
    }
}
