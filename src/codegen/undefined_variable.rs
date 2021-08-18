use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::reaching_definitions::{apply_transfers, VarDefs};
use crate::parser::pt::{Loc, StorageLocation};
use crate::sema::ast::{Diagnostic, ErrorType, Expression, Level, Namespace, Note};
use crate::sema::symtable;
use std::collections::HashMap;

/// We use this struct in expression.recurse function to provide all the
/// parameters for detecting undefined variables
pub struct FindUndefinedVariablesParams<'a> {
    pub func_no: usize,
    pub defs: &'a VarDefs,
    pub ns: &'a mut Namespace,
    pub cfg: &'a ControlFlowGraph,
    pub diagnostics: &'a mut HashMap<usize, Diagnostic>,
}

/// This function traverses all the instructions of each block, apply transfers and
/// look for undefined variables. Returns true if undefined variables have been detected
pub fn find_undefined_variables(
    cfg: &ControlFlowGraph,
    ns: &mut Namespace,
    func_no: usize,
) -> bool {
    let mut diagnostics: HashMap<usize, Diagnostic> = HashMap::new();
    for block in &cfg.blocks {
        let mut var_defs: VarDefs = block.defs.clone();
        for (instr_no, instruction) in block.instr.iter().enumerate() {
            check_variables_in_expression(
                func_no,
                instruction,
                &var_defs,
                ns,
                cfg,
                &mut diagnostics,
            );
            apply_transfers(&block.transfers[instr_no], &mut var_defs);
        }
    }

    let mut all_diagnostics: Vec<Diagnostic> =
        diagnostics.into_iter().map(|(_, diag)| diag).collect();
    ns.diagnostics.append(&mut all_diagnostics);

    !all_diagnostics.is_empty()
}

/// Checks for undefined variables in an expression associated to an instruction
pub fn check_variables_in_expression(
    func_no: usize,
    instr: &Instr,
    defs: &VarDefs,
    ns: &mut Namespace,
    cfg: &ControlFlowGraph,
    diagnostics: &mut HashMap<usize, Diagnostic>,
) {
    if matches!(instr, Instr::Store { .. }) {
        return;
    }

    let mut params = FindUndefinedVariablesParams {
        func_no,
        defs,
        ns,
        cfg,
        diagnostics,
    };
    instr.recurse_expressions(&mut params, find_undefined_variables_in_expression);
}

/// Auxiliar function for expression.recurse. It checks if a variable is read before being defined
pub fn find_undefined_variables_in_expression(
    exp: &Expression,
    ctx: &mut FindUndefinedVariablesParams,
) -> bool {
    match &exp {
        Expression::Variable(_, _, pos) => {
            if let (Some(def_map), Some(var)) = (
                ctx.defs.get(pos),
                ctx.ns.functions[ctx.func_no].symtable.vars.get(pos),
            ) {
                for (def, modified) in def_map {
                    if let Instr::Set {
                        expr: instr_expr, ..
                    } = &ctx.cfg.blocks[def.block_no].instr[def.instr_no]
                    {
                        // If an undefined definition reaches this read and the variable
                        // has not been modified since its definition, it is undefined
                        if matches!(instr_expr, Expression::Undefined(_)) && !*modified {
                            add_diagnostic(var, pos, &exp.loc(), ctx.diagnostics);
                        }
                    }
                }
            }
            false
        }

        // This is a method call whose array will never be undefined
        Expression::DynamicArrayLength(..) => false,

        _ => true,
    }
}

/// Add a diagnostic or a note to the hashmap. This function prevents duplicate
/// error messages in Diagnotics
fn add_diagnostic(
    var: &symtable::Variable,
    var_no: &usize,
    expr_loc: &Loc,
    diagnostics: &mut HashMap<usize, Diagnostic>,
) {
    if matches!(var.usage_type, symtable::VariableUsage::ReturnVariable)
        && !matches!(var.storage_location, Some(StorageLocation::Storage(_)))
    {
        return;
    }

    if !diagnostics.contains_key(var_no) {
        diagnostics.insert(
            *var_no,
            Diagnostic {
                level: Level::Error,
                ty: ErrorType::TypeError,
                pos: Some(var.id.loc),
                message: format!("Variable '{}' is undefined", var.id.name),
                notes: vec![],
            },
        );
    }

    let diag = diagnostics.get_mut(var_no).unwrap();
    diag.notes.push(Note {
        pos: *expr_loc,
        message: "Variable read before being defined".to_string(),
    });
}
