pub mod cfg;
mod expression;
mod statements;
mod storage;

use self::cfg::{ControlFlowGraph, Instr, Vartable};
use self::expression::expression;
use sema::ast::Namespace;

/// The contracts are fully resolved but they do not have any a CFG which is needed for the llvm code emitter
/// not all contracts need a cfg; only those for which we need the
pub fn codegen(contract_no: usize, ns: &mut Namespace) {
    for function_no in 0..ns.contracts[contract_no].functions.len() {
        let c = cfg::generate_cfg(contract_no, function_no, ns);
        ns.contracts[contract_no].functions[function_no].cfg = Some(c);
    }

    // Generate cfg for storage initializers
    ns.contracts[contract_no].initializer = storage_initializer(contract_no, ns);
}

/// This function will set all contract storage initializers and should be called from the constructor
fn storage_initializer(contract_no: usize, ns: &Namespace) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new();
    let mut vartab = Vartable::new();

    for var in &ns.contracts[contract_no].variables {
        if var.is_storage() {
            if let Some(init) = &var.initializer {
                let pos = vartab.temp_name(&var.name, &var.ty);
                let expr = expression(&init, &mut cfg, contract_no, ns, &mut vartab);
                cfg.add(&mut vartab, Instr::Set { res: pos, expr });
                cfg.add(
                    &mut vartab,
                    Instr::SetStorage {
                        local: pos,
                        ty: var.ty.clone(),
                        storage: var.get_storage_slot(),
                    },
                );
            }
        }
    }

    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    cfg.vars = vartab.drain();

    cfg
}
