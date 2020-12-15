use super::cfg::{BasicBlock, ControlFlowGraph, Instr};
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
    Kill { var_no: usize },
}

impl fmt::Display for Transfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Transfer::Gen { def, var_no } => {
                write!(f, "Gen %{} = ({}, {})", var_no, def.block_no, def.instr_no)
            }
            Transfer::Kill { var_no } => {
                write!(f, "Kill %{}", var_no)
            }
        }
    }
}

pub type VarDefs = HashMap<usize, HashSet<Def>>;

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
                        entry.extend(defs);
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
            Instr::Set { res, .. } => set_var(&[*res]),
            Instr::Call { res, .. } => set_var(res),
            Instr::AbiDecode { res, .. } => set_var(res),
            Instr::PushMemory { res, .. }
            | Instr::AbiEncodeVector { res, .. }
            | Instr::ExternalCall {
                success: Some(res), ..
            }
            | Instr::Constructor {
                success: None, res, ..
            } => set_var(&[*res]),
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

pub fn apply_transfers(transfers: &[Transfer], vars: &mut HashMap<usize, HashSet<Def>>) {
    for transfer in transfers {
        match transfer {
            Transfer::Kill { var_no } => {
                vars.remove(var_no);
            }
            Transfer::Gen { var_no, def } => {
                if let Some(entry) = vars.get_mut(var_no) {
                    entry.insert(*def);
                } else {
                    let mut v = HashSet::new();
                    v.insert(*def);
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
