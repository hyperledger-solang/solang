// SPDX-License-Identifier: Apache-2.0

use super::cfg::{BasicBlock, ControlFlowGraph, Instr};
use crate::codegen::Expression;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Def {
    pub block_no: usize,
    pub instr_no: usize,
    pub assignment_no: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    Gen { def: Def, var_no: usize },
    Mod { var_no: usize },
    Copy { var_no: usize, src: usize },
    Kill { var_no: usize },
}

impl fmt::Display for Transfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Transfer::Gen { def, var_no } => {
                write!(f, "Gen %{var_no} = ({}, {})", def.block_no, def.instr_no)
            }
            Transfer::Mod { var_no } => {
                write!(f, "Mod %{var_no}")
            }
            Transfer::Copy { var_no, src } => {
                write!(f, "Copy %{var_no} from %{src}")
            }
            Transfer::Kill { var_no } => {
                write!(f, "Kill %{var_no}")
            }
        }
    }
}

pub type VarDefs = IndexMap<usize, IndexMap<Def, bool>>;

/// Calculate all the reaching definitions for the contract. This is a flow
/// analysis which is used for further optimizations
pub fn find(cfg: &mut ControlFlowGraph) {
    // calculate the per-instruction reaching defs
    for (block_no, block) in cfg.blocks.iter_mut().enumerate() {
        block.transfers = instr_transfers(block_no, block);
    }

    let mut blocks_todo: HashSet<usize> = HashSet::new();
    blocks_todo.insert(0);

    while let Some(block_no) = blocks_todo.iter().next() {
        let block_no = *block_no;
        blocks_todo.remove(&block_no);

        let mut vars = cfg.blocks[block_no].defs.clone();

        for transfers in &cfg.blocks[block_no].transfers {
            apply_transfers(transfers, &mut vars);
        }

        for edge in cfg.blocks[block_no].successors() {
            if cfg.blocks[edge].defs != vars {
                blocks_todo.insert(edge);
                // merge incoming set
                for (var_no, defs) in &vars {
                    if let Some(entry) = cfg.blocks[edge].defs.get_mut(var_no) {
                        for (incoming_def, incoming_modified) in defs {
                            if let Some(e) = entry.get_mut(incoming_def) {
                                *e |= *incoming_modified;
                            } else {
                                entry.insert(*incoming_def, *incoming_modified);
                            }
                        }
                    } else {
                        cfg.blocks[edge].defs.insert(*var_no, defs.clone());
                    }

                    // If a definition from a block executed later reaches this block,
                    // this is a loop. This is an analysis we use later at the
                    // common subexpression elimination.
                    for (incoming_def, _) in defs {
                        if incoming_def.block_no >= edge {
                            cfg.blocks[edge].loop_reaching_variables.insert(*var_no);
                        }
                    }
                }
            }
        }
    }
}

/// Instruction defs
fn instr_transfers(block_no: usize, block: &BasicBlock) -> Vec<Vec<Transfer>> {
    let mut transfers = Vec::new();

    for (instr_no, instr) in block.instr.iter().enumerate() {
        let set_var = |var_nos: &[usize]| {
            let mut transfer = Vec::new();

            for (assignment_no, var_no) in var_nos.iter().enumerate() {
                transfer.insert(0, Transfer::Kill { var_no: *var_no });

                transfer.push(Transfer::Gen {
                    def: Def {
                        block_no,
                        instr_no,
                        assignment_no,
                    },
                    var_no: *var_no,
                });
            }

            transfer
        };

        transfers.push(match instr {
            Instr::Set {
                res,
                expr: Expression::Variable { var_no: src, .. },
                ..
            } => {
                vec![
                    Transfer::Kill { var_no: *res },
                    Transfer::Copy {
                        var_no: *res,
                        src: *src,
                    },
                ]
            }
            Instr::Set { res, .. } => set_var(&[*res]),
            Instr::Call { res, .. } => set_var(res),
            Instr::LoadStorage { res, .. } | Instr::PopStorage { res: Some(res), .. } => {
                set_var(&[*res])
            }
            Instr::PushMemory { array, res, .. } => {
                let mut v = set_var(&[*res]);
                v.push(Transfer::Mod { var_no: *array });

                v
            }
            Instr::PopMemory { array, .. } => {
                vec![Transfer::Mod { var_no: *array }]
            }
            Instr::ExternalCall {
                success: Some(res), ..
            }
            | Instr::Constructor {
                success: None, res, ..
            }
            | Instr::ValueTransfer {
                success: Some(res), ..
            } => set_var(&[*res]),
            Instr::ClearStorage { storage: dest, .. }
            | Instr::SetStorageBytes { storage: dest, .. }
            | Instr::SetStorage { storage: dest, .. }
            | Instr::Store { dest, .. } => {
                let mut v = Vec::new();

                if let Some(var_no) = array_var(dest) {
                    v.push(Transfer::Mod { var_no });
                }

                v
            }
            Instr::Constructor {
                success: Some(success),
                res,
                ..
            } => set_var(&[*res, *success]),
            _ => Vec::new(),
        });
    }

    transfers
}

fn array_var(expr: &Expression) -> Option<usize> {
    match expr {
        Expression::Variable { var_no, .. } => Some(*var_no),
        Expression::Subscript { expr, .. } | Expression::StructMember { expr, .. } => {
            array_var(expr)
        }
        _ => None,
    }
}

pub fn apply_transfers(transfers: &[Transfer], vars: &mut IndexMap<usize, IndexMap<Def, bool>>) {
    for transfer in transfers {
        match transfer {
            Transfer::Kill { var_no } => {
                vars.swap_remove(var_no);
            }
            Transfer::Mod { var_no } => {
                if let Some(entry) = vars.get_mut(var_no) {
                    for e in entry.values_mut() {
                        *e = true;
                    }
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
