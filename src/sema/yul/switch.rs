// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::Namespace;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use crate::sema::yul::ast::{CaseBlock, YulBlock, YulExpression, YulStatement};
use crate::sema::yul::block::resolve_yul_block;
use crate::sema::yul::expression::{check_type, resolve_yul_expression};
use crate::sema::yul::functions::FunctionsTable;
use crate::sema::yul::types::verify_type_from_expression;
use num_bigint::{BigInt, Sign};
use num_traits::{One, Zero};
use solang_parser::pt::{CodeLocation, YulSwitchOptions};
use solang_parser::{diagnostics::Diagnostic, pt};
use std::collections::HashMap;

/// Resolve switch statement
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
pub(crate) fn resolve_switch(
    yul_switch: &pt::YulSwitch,
    context: &mut ExprContext,
    reachable: bool,
    function_table: &mut FunctionsTable,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<(YulStatement, bool), ()> {
    let resolved_condition =
        resolve_condition(&yul_switch.condition, context, symtable, function_table, ns)?;
    let mut default_block: Option<YulBlock> = None;
    let mut case_blocks: Vec<CaseBlock> = Vec::with_capacity(yul_switch.cases.len());
    let mut next_reachable = reachable;
    for item in &yul_switch.cases {
        let block_reachable = resolve_case_or_default(
            item,
            &mut default_block,
            &mut case_blocks,
            context,
            next_reachable,
            function_table,
            loop_scope,
            symtable,
            ns,
        )?;
        next_reachable |= block_reachable;
    }

    let mut conditions: HashMap<BigInt, pt::Loc> = HashMap::new();
    for item in &case_blocks {
        let big_int = match &item.condition {
            YulExpression::BoolLiteral(_, value, _) => {
                if *value {
                    BigInt::one()
                } else {
                    BigInt::zero()
                }
            }
            YulExpression::NumberLiteral(_, value, _) => value.clone(),
            YulExpression::StringLiteral(_, value, _) => BigInt::from_bytes_be(Sign::Plus, value),
            _ => unreachable!("Switch condition should be a literal"),
        };

        let repeated_loc = conditions.get(&big_int);

        if let Some(repeated) = repeated_loc {
            ns.diagnostics.push(Diagnostic::error_with_note(
                item.condition.loc(),
                "duplicate case for switch".to_string(),
                *repeated,
                "repeated case found here".to_string(),
            ));
        } else {
            conditions.insert(big_int, item.condition.loc());
        }
    }

    if yul_switch.default.is_some() && default_block.is_some() {
        ns.diagnostics.push(Diagnostic::error(
            yul_switch.default.as_ref().unwrap().loc(),
            "Only one default block is allowed".to_string(),
        ));
        return Err(());
    } else if let Some(default_unwrapped) = &yul_switch.default {
        let block_reachable = resolve_case_or_default(
            default_unwrapped,
            &mut default_block,
            &mut case_blocks,
            context,
            next_reachable,
            function_table,
            loop_scope,
            symtable,
            ns,
        )?;
        next_reachable |= block_reachable;
    } else if yul_switch.default.is_none() && default_block.is_none() {
        next_reachable |= true;
    }

    Ok((
        YulStatement::Switch {
            loc: yul_switch.loc,
            reachable,
            condition: resolved_condition,
            cases: case_blocks,
            default: default_block,
        },
        next_reachable,
    ))
}

/// Resolves condition statements for either if-statement and switch-statements
pub(crate) fn resolve_condition(
    condition: &pt::YulExpression,
    context: &mut ExprContext,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
    let resolved_condition =
        resolve_yul_expression(condition, context, symtable, function_table, ns)?;
    if let Err(diagnostic) = verify_type_from_expression(&resolved_condition, function_table) {
        ns.diagnostics.push(diagnostic);
        return Err(());
    } else if let Some(diagnostic) = check_type(&resolved_condition, context, ns, symtable) {
        ns.diagnostics.push(diagnostic);
        return Err(());
    }

    Ok(resolved_condition)
}

/// Resolve case or default from a switch statements
fn resolve_case_or_default(
    switch_case: &pt::YulSwitchOptions,
    default_block: &mut Option<YulBlock>,
    case_blocks: &mut Vec<CaseBlock>,
    context: &mut ExprContext,
    reachable: bool,
    function_table: &mut FunctionsTable,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<bool, ()> {
    match switch_case {
        YulSwitchOptions::Case(loc, expr, block) => {
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

        YulSwitchOptions::Default(loc, block) => {
            let resolved_default = resolve_yul_block(
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
    condition: &pt::YulExpression,
    block: &[pt::YulStatement],
    context: &mut ExprContext,
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
        resolve_yul_expression(condition, context, symtable, function_table, ns)?;
    match resolved_condition {
        YulExpression::NumberLiteral(..)
        | YulExpression::StringLiteral(..)
        | YulExpression::BoolLiteral { .. } => (),

        _ => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_condition.loc(),
                "'case' can only be followed by a literal".to_string(),
            ));
            return Err(());
        }
    }

    let case_block = resolve_yul_block(
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
            loc: *loc,
            condition: resolved_condition,
            block: case_block.0,
        },
        case_block.1,
    ))
}
