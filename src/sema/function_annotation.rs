// SPDX-License-Identifier: Apache-2.0

use super::{
    ast::{ConstructorAnnotation, Diagnostic, Expression, Function, Namespace, Type},
    diagnostics::Diagnostics,
    eval::overflow_check,
    expression::literals::{hex_number_literal, unit_literal},
    expression::{ExprContext, ResolveTo},
    Symtable,
};
use crate::sema::ast::SolanaAccount;
use crate::sema::eval::eval_const_number;
use crate::sema::expression::literals::number_literal;
use crate::sema::expression::resolve_expression::expression;
use crate::sema::solana_accounts::BuiltinAccounts;
use crate::Target;
use indexmap::map::Entry;
use num_traits::ToPrimitive;
use solang_parser::pt::{self, Annotation, CodeLocation};
use std::str::FromStr;

/// Annotations are processed in two different places during sema. When we are resolving the
/// function header, we collect the parameter annotations in 'UnresolvedAnnotation' data structure.
/// Afterwards, during the function body resolution, we resolve the annotations.
pub(super) struct UnresolvedAnnotation {
    /// The parameter to which this annotation is attached.
    pub(super) parameter_no: usize,
    /// Variable number of this parameter in the symbol table.
    pub(super) var_no: usize,
}

/// This function simplifies the addition of a common error when we encounter a mispalced
/// annotation.
pub(super) fn unexpected_parameter_annotation(loc: pt::Loc) -> Diagnostic {
    Diagnostic::error(loc, "unexpected parameter annotation".to_string())
}

/// Resolve the prototype annotation for functions (just the selector). These
/// annotations can be resolved for functions without a body. This means they
/// do not need to access the symbol table, like `@seed(foo)` annotations do.
pub fn function_prototype_annotations(
    func: &mut Function,
    annotations: &[&pt::Annotation],
    ns: &mut Namespace,
) {
    let mut diagnostics = Diagnostics::default();

    for annotation in annotations {
        match annotation.id.name.as_str() {
            "selector" => function_selector(func, annotation, &mut diagnostics, ns),
            _ if !func.has_body => {
                // function_body_annotations() is called iff there is a body
                diagnostics.push(Diagnostic::error(
                    annotation.loc,
                    format!(
                        "annotation '@{}' not allowed on {} with no body",
                        annotation.id.name, func.ty
                    ),
                ));
            }
            _ => {
                // handled in function_body_annotations()
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}

/// Parse the selector from an annotation and assign it to the function
fn function_selector(
    func: &mut Function,
    annotation: &pt::Annotation,
    diagnostics: &mut Diagnostics,
    ns: &mut Namespace,
) {
    if func.ty != pt::FunctionTy::Function
        && (!ns.target.is_substrate() || func.ty != pt::FunctionTy::Constructor)
    {
        diagnostics.push(Diagnostic::error(
            annotation.loc,
            format!("overriding selector not permitted on {}", func.ty),
        ));
        return;
    }

    if !func.is_public() {
        diagnostics.push(Diagnostic::error(
            annotation.loc,
            format!(
                "overriding selector only permitted on 'public' or 'external' function, not '{}'",
                func.visibility
            ),
        ));
        return;
    }

    if let Some((prev, _)) = &func.selector {
        duplicate_annotation(
            diagnostics,
            "selector",
            annotation.loc,
            *prev,
            func.ty.as_str(),
        );
        return;
    }

    match &annotation.value.as_ref().unwrap() {
        pt::Expression::ArrayLiteral(_, values) => {
            let mut selector = Vec::new();

            for expr in values {
                let uint8 = Type::Uint(8);

                let expr = match expr {
                    pt::Expression::HexNumberLiteral(loc, n, None) => {
                        hex_number_literal(loc, n, ns, diagnostics, ResolveTo::Type(&uint8))
                    }
                    pt::Expression::NumberLiteral(loc, base, exp, unit) => {
                        let unit = unit_literal(loc, unit, ns, diagnostics);

                        number_literal(
                            loc,
                            base,
                            exp,
                            ns,
                            &unit,
                            diagnostics,
                            ResolveTo::Type(&uint8),
                        )
                    }
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            expr.loc(),
                            "literal number expected".into(),
                        ));
                        continue;
                    }
                };

                if let Ok(Expression::NumberLiteral { loc, value, .. }) = &expr {
                    if let Some(diagnostic) = overflow_check(value, &uint8, loc) {
                        diagnostics.push(diagnostic);
                    } else {
                        selector.push(value.to_u8().unwrap());
                    }
                } else {
                    // Diagnostic already generated
                    assert!(expr.is_err());
                }
            }

            if !diagnostics.any_errors() {
                func.selector = Some((annotation.loc, selector));
            }
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                annotation.value.as_ref().unwrap().loc(),
                "expression must be an array literal".into(),
            ));
        }
    }
}

/// Collect the seeds, bump, payer, and space for constructors. This is a no-op on Substrate/EVM since
/// there should be no seed or bump annotations permitted on other targets.
///
/// These annotations need a symbol table.
pub(super) fn function_body_annotations(
    function_no: usize,
    body_annotations: &[&pt::Annotation],
    parameter_annotations: &[UnresolvedAnnotation],
    symtable: &mut Symtable,
    context: &ExprContext,
    ns: &mut Namespace,
) {
    let mut diagnostics = Diagnostics::default();

    // On Solana, the seeds and bump for a constructor can be specified using annotations, for example
    //
    // @seed(param1)
    // @seed("fizbaz")
    // @bump(param2)
    // constructor(bytes param1, uint8 param2) {}

    let mut resolved_annotations = Vec::new();
    let mut bump = None;
    let mut space = None;
    let mut payer = None;

    let is_solana_constructor =
        ns.target == Target::Solana && ns.functions[function_no].ty == pt::FunctionTy::Constructor;

    for note in body_annotations {
        match note.id.name.as_str() {
            "selector" => {
                // selectors already done in function_prototype_annotations
                // without using a symbol table
            }
            "seed" if is_solana_constructor => {
                let ty = Type::Slice(Box::new(Type::Bytes(1)));

                let mut fake_loc = None;

                body_annotation(
                    note.id.name.as_str(),
                    &ty,
                    &mut fake_loc,
                    note,
                    &mut diagnostics,
                    &mut resolved_annotations,
                    context,
                    ns,
                    symtable,
                );
            }
            "bump" if is_solana_constructor => {
                let ty = Type::Bytes(1);

                body_annotation(
                    note.id.name.as_str(),
                    &ty,
                    &mut bump,
                    note,
                    &mut diagnostics,
                    &mut resolved_annotations,
                    context,
                    ns,
                    symtable,
                );
            }
            "space" if is_solana_constructor => {
                let ty = Type::Uint(64);

                body_annotation(
                    note.id.name.as_str(),
                    &ty,
                    &mut space,
                    note,
                    &mut diagnostics,
                    &mut resolved_annotations,
                    context,
                    ns,
                    symtable,
                );
            }
            "payer" if is_solana_constructor => {
                let loc = note.loc;
                if let pt::Expression::Variable(id) = note.value.as_ref().unwrap() {
                    if BuiltinAccounts::from_str(&id.name).is_ok() {
                        diagnostics.push(Diagnostic::error(
                            id.loc,
                            format!("'{}' is a reserved account name", id.name),
                        ));
                        continue;
                    }

                    match ns.functions[function_no]
                        .solana_accounts
                        .borrow_mut()
                        .entry(id.name.clone())
                    {
                        Entry::Occupied(other_account) => {
                            diagnostics.push(Diagnostic::error_with_note(
                                id.loc,
                                format!("account '{}' already defined", id.name),
                                other_account.get().loc,
                                "previous definition".to_string(),
                            ));
                        }
                        Entry::Vacant(vacancy) => {
                            if let Some(prev) = &payer {
                                duplicate_annotation(
                                    &mut diagnostics,
                                    "payer",
                                    loc,
                                    *prev,
                                    ns.functions[function_no].ty.as_str(),
                                );
                            } else {
                                payer = Some(loc);
                                vacancy.insert(SolanaAccount {
                                    loc: note.loc,
                                    is_signer: true,
                                    is_writer: true,
                                    generated: false,
                                });
                                resolved_annotations
                                    .push(ConstructorAnnotation::Payer(loc, id.name.clone()));
                            }
                        }
                    }
                } else {
                    diagnostics.push(Diagnostic::error(
                        note.loc,
                        "invalid parameter for annotation".to_string(),
                    ));
                }
            }
            _ => diagnostics.push(Diagnostic::error(
                note.loc,
                format!(
                    "unknown annotation {} for {}",
                    note.id.name, ns.functions[function_no].ty
                ),
            )),
        };
    }

    for unresolved in parameter_annotations {
        match ns.functions[function_no].params[unresolved.parameter_no]
            .annotation
            .as_ref()
            .unwrap()
            .id
            .name
            .as_str()
        {
            "seed" => {
                let ty = Type::Slice(Box::new(Type::Bytes(1)));
                let mut fake_loc = None;
                parameter_annotation(
                    function_no,
                    unresolved,
                    &ty,
                    &mut fake_loc,
                    &mut diagnostics,
                    &mut resolved_annotations,
                    ns,
                    symtable,
                );
            }
            "bump" => {
                let ty = Type::Bytes(1);
                parameter_annotation(
                    function_no,
                    unresolved,
                    &ty,
                    &mut bump,
                    &mut diagnostics,
                    &mut resolved_annotations,
                    ns,
                    symtable,
                );
            }
            "space" => {
                let ty = Type::Uint(64);
                parameter_annotation(
                    function_no,
                    unresolved,
                    &ty,
                    &mut space,
                    &mut diagnostics,
                    &mut resolved_annotations,
                    ns,
                    symtable,
                );
            }

            "payer" => {
                diagnostics.push(Diagnostic::error(
                    ns.functions[function_no].params[unresolved.parameter_no]
                        .annotation
                        .as_ref()
                        .unwrap()
                        .loc,
                    "@payer annotation not allowed next to a parameter".to_string(),
                ));
            }

            _ => {
                let annotation = ns.functions[function_no].params[unresolved.parameter_no]
                    .annotation
                    .as_ref()
                    .unwrap();
                diagnostics.push(Diagnostic::error(
                    annotation.loc,
                    format!(
                        "unknown annotation {} for {}",
                        annotation.id.name, ns.functions[function_no].ty
                    ),
                ))
            }
        }
    }

    if !resolved_annotations.is_empty() && diagnostics.is_empty() && payer.is_none() {
        diagnostics.push(Diagnostic::error(
            ns.functions[function_no].loc,
            "@payer annotation required for constructor".into(),
        ));
    }

    ns.diagnostics.extend(diagnostics);

    ns.functions[function_no].annotations = resolved_annotations;
}

/// Resolve the body annotations
fn body_annotation(
    name: &str,
    ty: &Type,
    previous: &mut Option<pt::Loc>,
    annotation: &Annotation,
    diagnostics: &mut Diagnostics,
    resolved_annotations: &mut Vec<ConstructorAnnotation>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) {
    let annotation_value = annotation.value.as_ref().unwrap();
    let mut dry_run = Diagnostics::default();
    if let Ok(expr) = expression(
        annotation_value,
        context,
        ns,
        symtable,
        &mut dry_run,
        ResolveTo::Type(ty),
    ) {
        // We only accept literals or constant expressions.
        if !annotation_value.is_literal() && eval_const_number(&expr, ns, &mut dry_run).is_err() {
            diagnostics.push(Diagnostic::error(
                annotation.value.as_ref().unwrap().loc(),
                format!(
                    "'@{}' annotation on top of a constructor only accepts literals",
                    name
                ),
            ));
            return;
        }
    }

    if let Ok(expr) = expression(
        annotation_value,
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(ty),
    ) {
        if let Ok(expr) = expr.cast(&expr.loc(), ty, true, ns, diagnostics) {
            if let Some(prev) = previous {
                duplicate_annotation(diagnostics, name, expr.loc(), *prev, "constructor");
            } else {
                *previous = Some(annotation.loc);
                resolved_annotations.push(ConstructorAnnotation::initialize_annotation(name, expr));
            }
        }
    }
}

/// Resolve parameter annotations
fn parameter_annotation(
    function_no: usize,
    unresolved_annotation: &UnresolvedAnnotation,
    ty: &Type,
    previous: &mut Option<pt::Loc>,
    diagnostics: &mut Diagnostics,
    resolved_annotations: &mut Vec<ConstructorAnnotation>,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) {
    let parameter = &ns.functions[function_no].params[unresolved_annotation.parameter_no];
    let annotation = parameter.annotation.as_ref().unwrap();
    if let Some(prev) = previous {
        duplicate_annotation(
            diagnostics,
            annotation.id.name.as_str(),
            annotation.loc,
            *prev,
            "constructor",
        );
        return;
    }

    let expr = Expression::Variable {
        loc: annotation.loc,
        ty: parameter.ty.clone(),
        var_no: unresolved_annotation.var_no,
    };

    // Mark variable as used, without using 'ns' (I cannot borrow it as mutable here)
    symtable
        .vars
        .get_mut(&unresolved_annotation.var_no)
        .unwrap()
        .read = true;

    if let Ok(casted) = expr.cast(&annotation.loc, ty, true, ns, diagnostics) {
        *previous = Some(annotation.loc);
        resolved_annotations.push(ConstructorAnnotation::initialize_annotation(
            annotation.id.name.as_str(),
            casted,
        ));
    }
}

impl ConstructorAnnotation {
    fn initialize_annotation(name: &str, value: Expression) -> ConstructorAnnotation {
        match name {
            "seed" => ConstructorAnnotation::Seed(value),
            "space" => ConstructorAnnotation::Space(value),
            "bump" => ConstructorAnnotation::Bump(value),
            _ => unreachable!("function should not be called with {}", name),
        }
    }
}

/// This function centralizes where we generate the duplicate annotation error.
fn duplicate_annotation(
    diagnostics: &mut Diagnostics,
    name: &str,
    new_loc: pt::Loc,
    old_loc: pt::Loc,
    func_ty: &str,
) {
    diagnostics.push(Diagnostic::error_with_note(
        new_loc,
        format!("duplicate @{} annotation for {}", name, func_ty),
        old_loc,
        format!("previous @{}", name),
    ));
}
