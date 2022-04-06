pub(crate) mod cfg;
mod constant_folding;
mod dead_storage;
mod expression;
mod external_functions;
mod reaching_definitions;
mod statements;
mod storage;
mod strength_reduce;
pub(crate) mod subexpression_elimination;
mod undefined_variable;
mod unused_variable;
pub(crate) mod vartable;
mod vector_to_slice;

use self::{
    cfg::{optimize_and_check_cfg, ControlFlowGraph, Instr},
    expression::expression,
    vartable::Vartable,
};
#[cfg(feature = "llvm")]
use crate::emit::Generate;
use crate::sema::ast::{Layout, Namespace};
use crate::sema::contracts::visit_bases;
use crate::sema::diagnostics::any_errors;
use crate::Target;

use crate::ast::Function;
use crate::codegen::cfg::ASTFunction;
use num_bigint::BigInt;
use num_traits::Zero;

// The sizeof(struct account_data_header)
pub const SOLANA_FIRST_OFFSET: u64 = 16;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum OptimizationLevel {
    None = 0,
    Less = 1,
    Default = 2,
    Aggressive = 3,
}

#[cfg(feature = "llvm")]
impl From<OptimizationLevel> for inkwell::OptimizationLevel {
    fn from(level: OptimizationLevel) -> Self {
        match level {
            OptimizationLevel::None => inkwell::OptimizationLevel::None,
            OptimizationLevel::Less => inkwell::OptimizationLevel::Less,
            OptimizationLevel::Default => inkwell::OptimizationLevel::Default,
            OptimizationLevel::Aggressive => inkwell::OptimizationLevel::Aggressive,
        }
    }
}

#[cfg(feature = "llvm")]
impl From<inkwell::OptimizationLevel> for OptimizationLevel {
    fn from(level: inkwell::OptimizationLevel) -> Self {
        match level {
            inkwell::OptimizationLevel::None => OptimizationLevel::None,
            inkwell::OptimizationLevel::Less => OptimizationLevel::Less,
            inkwell::OptimizationLevel::Default => OptimizationLevel::Default,
            inkwell::OptimizationLevel::Aggressive => OptimizationLevel::Aggressive,
        }
    }
}

pub struct Options {
    pub dead_storage: bool,
    pub constant_folding: bool,
    pub strength_reduce: bool,
    pub vector_to_slice: bool,
    pub math_overflow_check: bool,
    pub common_subexpression_elimination: bool,
    pub opt_level: OptimizationLevel,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            dead_storage: true,
            constant_folding: true,
            strength_reduce: true,
            vector_to_slice: true,
            math_overflow_check: false,
            common_subexpression_elimination: true,
            opt_level: OptimizationLevel::Default,
        }
    }
}

/// The contracts are fully resolved but they do not have any CFGs which is needed for
/// the llvm code emitter. This will also do additional code checks.
pub fn codegen(ns: &mut Namespace, opt: &Options) {
    if any_errors(&ns.diagnostics) {
        return;
    }

    let mut contracts_done = Vec::new();

    contracts_done.resize(ns.contracts.len(), false);

    // codegen all the contracts; some additional errors/warnings will be detected here
    while contracts_done.iter().any(|e| !*e) {
        for contract_no in 0..ns.contracts.len() {
            if contracts_done[contract_no] {
                continue;
            }

            if !ns.contracts[contract_no].is_concrete() {
                contracts_done[contract_no] = true;
                continue;
            }

            // does this contract create any contract which are not done
            if ns.contracts[contract_no]
                .creates
                .iter()
                .any(|c| !contracts_done[*c])
            {
                continue;
            }

            contract(contract_no, ns, opt);

            if any_errors(&ns.diagnostics) {
                return;
            }

            // Solana creates a single bundle
            if ns.target != Target::Solana {
                #[cfg(not(feature = "llvm"))]
                panic!("LLVM feature is not enabled");
                #[cfg(feature = "llvm")]
                {
                    let context = inkwell::context::Context::create();

                    let filename = ns.files[0].path.to_string_lossy();

                    let binary = ns.contracts[contract_no].emit(
                        ns,
                        &context,
                        &filename,
                        opt.opt_level.into(),
                        opt.math_overflow_check,
                    );

                    let code = binary.code(Generate::Linked).expect("llvm build");

                    drop(binary);

                    ns.contracts[contract_no].code = code;
                }
            }

            contracts_done[contract_no] = true;
        }
    }
}

fn contract(contract_no: usize, ns: &mut Namespace, opt: &Options) {
    if !any_errors(&ns.diagnostics) && ns.contracts[contract_no].is_concrete() {
        layout(contract_no, ns);

        let mut cfg_no = 0;
        let mut all_cfg = Vec::new();

        external_functions::add_external_functions(contract_no, ns);

        // all the functions should have a cfg_no assigned, so we can generate call instructions to the correct function
        for (_, func_cfg) in ns.contracts[contract_no].all_functions.iter_mut() {
            *func_cfg = cfg_no;
            cfg_no += 1;
        }

        // TODO: This should be done when we create a CFG for yul functions, which is not the case yet
        // for yul_fn_no in &ns.contracts[contract_no].yul_functions {
        //     ns.yul_functions[*yul_fn_no].cfg_no = cfg_no;
        //     cfg_no += 1;
        // }

        all_cfg.resize(cfg_no, ControlFlowGraph::placeholder());

        // clone all_functions so we can pass a mutable reference to generate_cfg
        for (function_no, cfg_no) in ns.contracts[contract_no]
            .all_functions
            .iter()
            .map(|(function_no, cfg_no)| (*function_no, *cfg_no))
            .collect::<Vec<(usize, usize)>>()
            .into_iter()
        {
            cfg::generate_cfg(
                contract_no,
                Some(function_no),
                cfg_no,
                &mut all_cfg,
                ns,
                opt,
            )
        }

        // for yul_func_no in &ns.contracts[contract_no].yul_functions {
        //     // TODO: Generate Yul function CFG
        // }

        // Generate cfg for storage initializers
        let cfg = storage_initializer(contract_no, ns, opt);
        let pos = all_cfg.len();
        all_cfg.push(cfg);
        ns.contracts[contract_no].initializer = Some(pos);

        if !ns.contracts[contract_no].have_constructor(ns) {
            // generate the default constructor
            let func = ns.default_constructor(contract_no);
            let cfg_no = all_cfg.len();
            all_cfg.push(ControlFlowGraph::placeholder());

            cfg::generate_cfg(contract_no, None, cfg_no, &mut all_cfg, ns, opt);

            ns.contracts[contract_no].default_constructor = Some((func, cfg_no));
        }

        ns.contracts[contract_no].cfg = all_cfg;
    }
}

/// This function will set all contract storage initializers and should be called from the constructor
fn storage_initializer(contract_no: usize, ns: &mut Namespace, opt: &Options) -> ControlFlowGraph {
    // note the single `:` to prevent a name clash with user-declared functions
    let mut cfg = ControlFlowGraph::new(
        format!("{}:storage_initializer", ns.contracts[contract_no].name),
        ASTFunction::None,
    );
    let mut vartab = Vartable::new(ns.next_id);

    for layout in &ns.contracts[contract_no].layout {
        let var = &ns.contracts[layout.contract_no].variables[layout.var_no];

        if let Some(init) = &var.initializer {
            let storage =
                ns.contracts[contract_no].get_storage_slot(layout.contract_no, layout.var_no, ns);

            let value = expression(init, &mut cfg, contract_no, None, ns, &mut vartab, opt);

            cfg.add(
                &mut vartab,
                Instr::SetStorage {
                    value,
                    ty: var.ty.clone(),
                    storage,
                },
            );
        }
    }

    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    let (vars, next_id) = vartab.drain();
    cfg.vars = vars;
    ns.next_id = next_id;

    optimize_and_check_cfg(&mut cfg, ns, None, opt);

    cfg
}

/// Layout the contract. We determine the layout of variables and deal with overriding variables
fn layout(contract_no: usize, ns: &mut Namespace) {
    let mut slot = if ns.target == Target::Solana {
        BigInt::from(SOLANA_FIRST_OFFSET)
    } else {
        BigInt::zero()
    };

    for base_contract_no in visit_bases(contract_no, ns) {
        for var_no in 0..ns.contracts[base_contract_no].variables.len() {
            if !ns.contracts[base_contract_no].variables[var_no].constant {
                let ty = ns.contracts[base_contract_no].variables[var_no].ty.clone();

                if ns.target == Target::Solana {
                    // elements need to be aligned on solana
                    let alignment = ty.align_of(ns);

                    let offset = slot.clone() % alignment;

                    if offset > BigInt::zero() {
                        slot += alignment - offset;
                    }
                }

                ns.contracts[contract_no].layout.push(Layout {
                    slot: slot.clone(),
                    contract_no: base_contract_no,
                    var_no,
                    ty: ty.clone(),
                });

                slot += ty.storage_slots(ns);
            }
        }
    }

    ns.contracts[contract_no].fixed_layout_size = slot;
}

trait LLVMName {
    fn llvm_symbol(&self, ns: &Namespace) -> String;
}

impl LLVMName for Function {
    /// Return a unique string for this function which is a valid llvm symbol
    fn llvm_symbol(&self, ns: &Namespace) -> String {
        let mut sig = self.name.to_owned();

        if !self.params.is_empty() {
            sig.push_str("__");

            for (i, p) in self.params.iter().enumerate() {
                if i > 0 {
                    sig.push('_');
                }

                sig.push_str(&p.ty.to_llvm_string(ns));
            }
        }

        sig
    }
}
