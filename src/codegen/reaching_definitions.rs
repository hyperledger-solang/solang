use super::cfg::{BasicBlock, ControlFlowGraph, Instr};
use crate::sema::ast::Expression;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Def {
    pub block_no: usize,
    pub instr_no: usize,
}

#[derive(Clone, Copy, PartialEq)]
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
                write!(f, "Gen %{} = ({}, {})", var_no, def.block_no, def.instr_no)
            }
            Transfer::Mod { var_no } => {
                write!(f, "Mod %{}", var_no)
            }
            Transfer::Copy { var_no, src } => {
                write!(f, "Copy %{} from %{}", var_no, src)
            }
            Transfer::Kill { var_no } => {
                write!(f, "Kill %{}", var_no)
            }
        }
    }
}

pub type VarDefs = HashMap<usize, HashMap<Def, bool>>;

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

        for edge in block_edges(&cfg.blocks[block_no]) {
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
                }
            }
        }
    }
}

/// Instruction defs
fn instr_transfers(block_no: usize, block: &BasicBlock) -> Vec<Vec<Transfer>> {
    let mut transfers = Vec::new();

    for (instr_no, instr) in block.instr.iter().enumerate() {
        let def = Def { block_no, instr_no };

        let set_var = |var_nos: &[usize]| {
            let mut transfer = Vec::new();

            for var_no in var_nos.iter() {
                transfer.insert(0, Transfer::Kill { var_no: *var_no });

                transfer.push(Transfer::Gen {
                    def,
                    var_no: *var_no,
                });
            }

            transfer
        };

        transfers.push(match instr {
            Instr::Set {
                res,
                expr: Expression::Variable(_, _, src),
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
            Instr::AbiDecode { res, .. } => set_var(res),
            Instr::PopStorage { res, .. } => set_var(&[*res]),
            Instr::PushMemory { array, res, .. } => {
                let mut v = set_var(&[*res]);
                v.push(Transfer::Mod { var_no: *array });

                v
            }
            Instr::PopMemory { array, .. } => {
                vec![Transfer::Mod { var_no: *array }]
            }
            Instr::AbiEncodeVector { res, .. }
            | Instr::ExternalCall {
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
        Expression::Variable(_, _, var_no) => Some(*var_no),
        Expression::DynamicArraySubscript(_, _, expr, _)
        | Expression::Subscript(_, _, expr, _)
        | Expression::StructMember(_, _, expr, _) => array_var(expr),
        _ => None,
    }
}

pub fn apply_transfers(transfers: &[Transfer], vars: &mut HashMap<usize, HashMap<Def, bool>>) {
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
                    let mut v = HashMap::new();
                    v.insert(*def, false);
                    vars.insert(*var_no, v);
                }
            }
        }
    }
}

pub fn block_edges(block: &BasicBlock) -> Vec<usize> {
    let mut out = Vec::new();

    // out cfg has edge as the last instruction in a block; EXCEPT
    // Instr::AbiDecode() which has an edge when decoding fails
    for instr in &block.instr {
        match instr {
            Instr::Branch { block } => {
                out.push(*block);
            }
            Instr::BranchCond {
                true_block,
                false_block,
                ..
            } => {
                out.push(*true_block);
                out.push(*false_block);
            }
            Instr::AbiDecode {
                exception_block: Some(block),
                ..
            } => {
                out.push(*block);
            }
            _ => (),
        }
    }

    out
}
