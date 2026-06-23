// SPDX-License-Identifier: Apache-2.0

use super::expression_values::expression_values;
use super::value::Value;
use super::{track, Variables, MAX_VALUES};
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::sema::ast::Namespace;
use std::collections::{HashMap, HashSet};

/// Step through a block, and calculate the reaching values for all the variables
pub(super) fn reaching_values(
    block_no: usize,
    cfg: &ControlFlowGraph,
    vars: &mut Variables,
    block_vars: &mut HashMap<usize, Variables>,
    ns: &Namespace,
) {
    // We should merge the incoming set of variables with the existing ones. If there
    // are no changes, then we cease traversing the cfg. The rules are:
    // - If there are more than MAX_VALUES entries in the result, make the set the unknown set
    // - If either the existing set or the incoming set contains unknown, make set the unknown set
    // - If there are no changes to the existing set, record this
    // - This is a very hot code path. This needs to be _FAST_ else compilation time quickly explodes
    if let Some(map) = block_vars.get_mut(&block_no) {
        let mut changes = false;

        for (var_no, set) in vars.iter() {
            changes |= update_map(*var_no, set, map);
        }

        if !changes {
            // no need to do this again
            return;
        }
    } else {
        block_vars.insert(block_no, vars.clone());
    }

    for instr in &cfg.blocks[block_no].instr {
        transfer(instr, vars, ns);

        match instr {
            Instr::Branch { block } => {
                // must be last in the block
                reaching_values(*block, cfg, vars, block_vars, ns);
            }
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => {
                // must be last in the block
                let v = expression_values(cond, vars, ns);

                if v.len() == 1 {
                    let v = v.iter().next().unwrap();

                    // if we know the value of the condition, follow that path
                    if v.known_bits[0] {
                        reaching_values(
                            if v.value[0] {
                                *true_block
                            } else {
                                *false_block
                            },
                            cfg,
                            vars,
                            block_vars,
                            ns,
                        );

                        continue;
                    }
                }

                // we don't know the value of the condition. Follow both paths
                let mut vars_copy = vars.clone();

                reaching_values(*true_block, cfg, &mut vars_copy, block_vars, ns);

                reaching_values(*false_block, cfg, vars, block_vars, ns);
            }
            _ => (),
        }
    }
}

/// Update the Variable's map based on the incoming set of values. Returns true if there was any
/// changes in the set.
/// There is a discussion to improve this function: https://github.com/hyperledger-solang/solang/issues/934
fn update_map(var_no: usize, set: &HashSet<Value>, map: &mut Variables) -> bool {
    if let Some(existing) = map.get_mut(&var_no) {
        if existing.iter().next().is_some_and(|v| v.all_unknown()) {
            // If we already think it is unknown, nothing can improve on that
            false
        } else if let Some(v) = set.iter().find(|v| v.all_unknown()) {
            // If we are merging an unknown value, set the entire value set to unknown
            let mut set = HashSet::new();

            set.insert(v.clone());

            map.insert(var_no, set);
            true
        } else {
            let mut changes = false;
            for v in set {
                if !existing.contains(v) {
                    existing.insert(v.clone());
                    changes = true;
                }
            }

            if existing.len() > MAX_VALUES {
                let bits = existing.iter().next().unwrap().bits;

                let mut set = HashSet::new();

                set.insert(Value::unknown(bits));

                changes = true;
                map.insert(var_no, set);
            }
            changes
        }
    } else {
        // We have no existing set. Create one but folding unknown

        if set.len() > MAX_VALUES || set.iter().any(|v| v.all_unknown()) {
            let bits = set.iter().next().unwrap().bits;

            let mut set = HashSet::new();

            set.insert(Value::unknown(bits));

            map.insert(var_no, set);
        } else {
            map.insert(var_no, set.clone());
        }

        true
    }
}

/// For a given instruction, calculate the new reaching values
pub(super) fn transfer(instr: &Instr, vars: &mut Variables, ns: &Namespace) {
    match instr {
        Instr::Set { res, expr, .. } => {
            let v = expression_values(expr, vars, ns);

            vars.insert(*res, v);
        }
        Instr::Call {
            res, return_tys, ..
        } => {
            for (i, var_no) in res.iter().enumerate() {
                let mut set = HashSet::new();

                let ty = &return_tys[i];

                if track(ty) {
                    let bits = ty.bits(ns) as usize;

                    set.insert(Value::unknown(bits));

                    vars.insert(*var_no, set);
                }
            }
        }
        Instr::PopStorage { res: Some(res), .. } => {
            let mut set = HashSet::new();

            set.insert(Value::unknown(8));

            vars.insert(*res, set);
        }
        Instr::PopMemory { res, ty, .. } => {
            if track(ty) {
                let mut set = HashSet::new();

                let bits = ty.bits(ns) as usize;

                set.insert(Value::unknown(bits));

                vars.insert(*res, set);
            }
        }
        _ => (),
    }
}
