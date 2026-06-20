// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ASTFunction, ControlFlowGraph, Instr};
use crate::codegen::reaching_definitions::{apply_transfers, VarDefs};
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{Diagnostic, ErrorType, Level, Namespace, Note, Type};
use crate::sema::symtable;
use solang_parser::pt::CodeLocation;
use solang_parser::pt::{Loc, StorageLocation};
use std::collections::HashMap;

/// We use this struct in expression.recurse function to provide all the
/// parameters for detecting undefined variables
pub struct FindUndefinedVariablesParams<'a> {
    pub func_no: ASTFunction,
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
    func_no: ASTFunction,
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

    let mut all_diagnostics: Vec<Diagnostic> = diagnostics.into_values().collect();
    let has_diagnostic = !all_diagnostics.is_empty();
    ns.diagnostics.append(&mut all_diagnostics);

    has_diagnostic
}

/// Checks for undefined variables in an expression associated to an instruction
pub fn check_variables_in_expression(
    func_no: ASTFunction,
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
        Expression::Variable { var_no, .. } => {
            let variable = match ctx.func_no {
                ASTFunction::YulFunction(func_no) => {
                    ctx.ns.yul_functions[func_no].symtable.vars.get(var_no)
                }
                ASTFunction::SolidityFunction(func_no) => {
                    ctx.ns.functions[func_no].symtable.vars.get(var_no)
                }

                ASTFunction::None => None,
            };

            if let (Some(def_map), Some(var)) = (ctx.defs.get(var_no), variable) {
                for (def, modified) in def_map {
                    if let Instr::Set {
                        expr: instr_expr, ..
                    } = &ctx.cfg.blocks[def.block_no].instr[def.instr_no]
                    {
                        // If an undefined definition reaches this read and the variable
                        // has not been modified since its definition, it is undefined
                        if matches!(instr_expr, Expression::Undefined { .. })
                            && !*modified
                            && !matches!(var.ty, Type::Array(..))
                        {
                            add_diagnostic(var, *var_no, &exp.loc(), ctx.diagnostics);
                        }
                    }
                }
            }
            false
        }

        // This is a method call whose array will never be undefined
        Expression::Builtin {
            kind: Builtin::ArrayLength,
            ..
        } => false,

        _ => true,
    }
}

/// Add a diagnostic or a note to the hashmap. This function prevents duplicate
/// error messages in Diagnotics
fn add_diagnostic(
    var: &symtable::Variable,
    var_no: usize,
    expr_loc: &Loc,
    diagnostics: &mut HashMap<usize, Diagnostic>,
) {
    if matches!(var.usage_type, symtable::VariableUsage::ReturnVariable)
        && !matches!(var.storage_location, Some(StorageLocation::Storage(_)))
    {
        return;
    }

    diagnostics.entry(var_no).or_insert(Diagnostic {
        level: Level::Error,
        ty: ErrorType::TypeError,
        loc: var.id.loc,
        message: format!("Variable '{}' is undefined", var.id.name),
        notes: vec![],
    });

    let diag = diagnostics.get_mut(&var_no).unwrap();
    diag.notes.push(Note {
        loc: *expr_loc,
        message: "Variable read before being defined".to_string(),
    });
}

// TODO: undefined variables are not yet compatible with Yul
