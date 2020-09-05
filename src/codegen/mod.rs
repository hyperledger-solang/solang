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
    if ns.contracts[contract_no].is_concrete() {
        let mut cfg_no = 0;
        let mut all_cfg = Vec::new();

        // all the functions should have a cfg_no assigned, so we can generate call instructions to the correct function
        for (_, func_cfg) in ns.contracts[contract_no].all_functions.iter_mut() {
            *func_cfg = cfg_no;
            cfg_no += 1;
        }

        all_cfg.resize(cfg_no, ControlFlowGraph::placeholder());

        // clone all_functions so we can pass a mutable reference to generate_cfg
        for (base_contract_no, function_no, cfg_no) in ns.contracts[contract_no]
            .all_functions
            .iter()
            .map(|((base_contract_no, function_no), cfg_no)| {
                (*base_contract_no, *function_no, *cfg_no)
            })
            .collect::<Vec<(usize, usize, usize)>>()
            .into_iter()
        {
            cfg::generate_cfg(
                contract_no,
                base_contract_no,
                Some(function_no),
                cfg_no,
                &mut all_cfg,
                ns,
            )
        }

        // Generate cfg for storage initializers
        let cfg = storage_initializer(contract_no, ns);
        let pos = all_cfg.len();
        all_cfg.push(cfg);
        ns.contracts[contract_no].initializer = Some(pos);

        if !ns.contracts[contract_no].have_constructor() {
            // generate the default constructor
            let func = ns.default_constructor();
            let cfg_no = all_cfg.len();
            all_cfg.push(ControlFlowGraph::placeholder());

            cfg::generate_cfg(contract_no, contract_no, None, cfg_no, &mut all_cfg, ns);

            ns.contracts[contract_no].default_constructor = Some((func, cfg_no));
        }

        ns.contracts[contract_no].cfg = all_cfg;
    }
}

/// This function will set all contract storage initializers and should be called from the constructor
fn storage_initializer(contract_no: usize, ns: &Namespace) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new(String::from("storage_initializer"));
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
