pub mod cfg;
mod expression;
mod statements;
mod storage;

use self::cfg::{ControlFlowGraph, Instr, Vartable};
use self::expression::expression;
use sema::ast::Namespace;
use std::collections::HashMap;

/// The contracts are fully resolved but they do not have any a CFG which is needed for the llvm code emitter
/// not all contracts need a cfg; only those for which we need the
pub fn codegen(contract_no: usize, ns: &mut Namespace) {
    if ns.contracts[contract_no].is_concrete() {
        // we need to iterate over the contracts function table, while generating cfg for each. We can't
        // iterate mutably over the function table because then we cannot borrow the namespace for generating
        // the cfg. So first populate a separate hash and then set in another loop
        let mut generated = HashMap::new();

        for (signature, con) in &ns.contracts[contract_no].function_table {
            let c = cfg::generate_cfg(contract_no, con.0, Some(con.1), ns);
            generated.insert(signature.to_owned(), c);
        }

        for (signature, con) in ns.contracts[contract_no].function_table.iter_mut() {
            con.2 = generated.remove(signature);
        }

        // Generate cfg for storage initializers
        ns.contracts[contract_no].initializer = storage_initializer(contract_no, ns);

        if !ns.contracts[contract_no].have_constructor() {
            // generate the default constructor
            let func = ns.default_constructor(contract_no);

            ns.contracts[contract_no].default_constructor =
                Some((func, cfg::generate_cfg(contract_no, contract_no, None, ns)));
        }
    }
}

/// This function will set all contract storage initializers and should be called from the constructor
fn storage_initializer(contract_no: usize, ns: &Namespace) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new();
    let mut vartab = Vartable::new(ns.next_id);

    for layout in &ns.contracts[contract_no].layout {
        let var = &ns.contracts[layout.contract_no].variables[layout.var_no];

        if let Some(init) = &var.initializer {
            let storage =
                ns.contracts[contract_no].get_storage_slot(layout.contract_no, layout.var_no);

            let pos = vartab.temp_name(&var.name, &var.ty);
            let expr = expression(&init, &mut cfg, contract_no, ns, &mut vartab);
            cfg.add(&mut vartab, Instr::Set { res: pos, expr });
            cfg.add(
                &mut vartab,
                Instr::SetStorage {
                    local: pos,
                    ty: var.ty.clone(),
                    storage,
                },
            );
        }
    }

    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    cfg.vars = vartab.drain();

    cfg
}
