use crate::ast::Namespace;
use crate::sema::assembly::block::{resolve_assembly_block, AssemblyBlock};
use crate::sema::assembly::expression::{
    check_type, resolve_assembly_expression, AssemblyExpression,
};
use crate::sema::assembly::functions::FunctionsTable;
use crate::sema::assembly::statements::AssemblyStatement;
use crate::sema::assembly::types::verify_type_from_expression;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use solang_parser::pt::{AssemblySwitchOptions, CodeLocation};
use solang_parser::{pt, Diagnostic};

#[derive(Debug, Clone)]
pub struct CaseBlock {
    pub condition: AssemblyExpression,
    pub block: AssemblyBlock,
}

/// Resolve switch statement
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
pub(crate) fn resolve_switch(
    assembly_switch: &pt::AssemblySwitch,
    context: &ExprContext,
    mut reachable: bool,
    function_table: &mut FunctionsTable,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<(AssemblyStatement, bool), ()> {
    let resolved_condition = resolve_condition(
        &assembly_switch.condition,
        context,
        symtable,
        function_table,
        ns,
    )?;
    let mut default_block: Option<AssemblyBlock> = None;
    let mut case_blocks: Vec<CaseBlock> = Vec::with_capacity(assembly_switch.cases.len());
    for item in &assembly_switch.cases {
        let block_reachable = resolve_case_or_default(
            item,
            &mut default_block,
            &mut case_blocks,
            context,
            reachable,
            function_table,
            loop_scope,
            symtable,
            ns,
        )?;
        reachable |= block_reachable;
    }

    if assembly_switch.default.is_some() && default_block.is_some() {
        ns.diagnostics.push(Diagnostic::error(
            assembly_switch.default.as_ref().unwrap().loc(),
            "Only one default block is allowed".to_string(),
        ));
        return Err(());
    } else if let Some(default_unwrapped) = &assembly_switch.default {
        let block_reachable = resolve_case_or_default(
            default_unwrapped,
            &mut default_block,
            &mut case_blocks,
            context,
            reachable,
            function_table,
            loop_scope,
            symtable,
            ns,
        )?;
        reachable |= block_reachable;
    } else if assembly_switch.default.is_none() && default_block.is_none() {
        reachable |= true;
    }

    Ok((
        AssemblyStatement::Switch {
            loc: assembly_switch.loc,
            condition: resolved_condition,
            cases: case_blocks,
            default: default_block,
        },
        reachable,
    ))
}

/// Resolves condition statements for either if-statement and switch-statements
pub(crate) fn resolve_condition(
    condition: &pt::AssemblyExpression,
    context: &ExprContext,
    symtable: &Symtable,
    function_table: &FunctionsTable,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    let resolved_condition =
        resolve_assembly_expression(condition, context, symtable, function_table, ns)?;
    if let Err(diagnostic) = verify_type_from_expression(&resolved_condition, function_table) {
        ns.diagnostics.push(diagnostic);
        return Err(());
    } else if let Some(diagnostic) = check_type(&resolved_condition, context) {
        ns.diagnostics.push(diagnostic);
        return Err(());
    }

    Ok(resolved_condition)
}

/// Resolve case or default from a switch statements
fn resolve_case_or_default(
    switch_case: &pt::AssemblySwitchOptions,
    default_block: &mut Option<AssemblyBlock>,
    case_blocks: &mut Vec<CaseBlock>,
    context: &ExprContext,
    reachable: bool,
    function_table: &mut FunctionsTable,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<bool, ()> {
    match switch_case {
        AssemblySwitchOptions::Case(loc, expr, block) => {
            let resolved_case = resolve_case_block(
                loc,
                default_block.is_some(),
                expr,
                &block.statements,
                context,
                reachable,
                function_table,
                loop_scope,
                symtable,
                ns,
            )?;
            case_blocks.push(resolved_case.0);
            Ok(resolved_case.1)
        }

        AssemblySwitchOptions::Default(loc, block) => {
            let resolved_default = resolve_assembly_block(
                loc,
                &block.statements,
                context,
                reachable,
                loop_scope,
                function_table,
                symtable,
                ns,
            );
            *default_block = Some(resolved_default.0);
            Ok(resolved_default.1)
        }
    }
}

/// Resolve a case from a switch statement
fn resolve_case_block(
    loc: &pt::Loc,
    has_default: bool,
    condition: &pt::AssemblyExpression,
    block: &[pt::AssemblyStatement],
    context: &ExprContext,
    reachable: bool,
    function_table: &mut FunctionsTable,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<(CaseBlock, bool), ()> {
    if has_default {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "A case-block cannot come after a default block".to_string(),
        ));
        return Err(());
    }
    let resolved_condition =
        resolve_assembly_expression(condition, context, symtable, function_table, ns)?;
    match resolved_condition {
        AssemblyExpression::NumberLiteral(..)
        | AssemblyExpression::StringLiteral(..)
        | AssemblyExpression::BoolLiteral(..) => (),

        _ => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_condition.loc(),
                "'case' can only be followed by a literal".to_string(),
            ));
            return Err(());
        }
    }

    let case_block = resolve_assembly_block(
        loc,
        block,
        context,
        reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );

    Ok((
        CaseBlock {
            condition: resolved_condition,
            block: case_block.0,
        },
        case_block.1,
    ))
}
