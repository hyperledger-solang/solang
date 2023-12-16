// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{
    ArrayLength, Builtin, Expression, Namespace, RetrieveType, StructType, Symbol, Type,
};
use crate::sema::builtin;
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::constructor::circular_reference;
use crate::sema::expression::function_call::function_type;
use crate::sema::expression::integers::bigint_to_expression;
use crate::sema::expression::resolve_expression::expression;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::solana_accounts::BuiltinAccounts;
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::{assigned_variable, used_variable};
use crate::Target;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use solang_parser::diagnostics::{Diagnostic, Note};
use solang_parser::pt;
use solang_parser::pt::CodeLocation;

/// Resolve an member access expression
pub(super) fn member_access(
    loc: &pt::Loc,
    e: &pt::Expression,
    id: &pt::Identifier,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    // is it a builtin special variable like "block.timestamp"
    if let pt::Expression::Variable(namespace) = e {
        if let Some((builtin, ty)) =
            builtin::builtin_var(loc, Some(&namespace.name), &id.name, ns, diagnostics)
        {
            return Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![ty],
                kind: builtin,
                args: vec![],
            });
        }

        if builtin::builtin_namespace(&namespace.name) {
            diagnostics.push(Diagnostic::error(
                e.loc(),
                format!("builtin '{}.{}' does not exist", namespace.name, id.name),
            ));

            return Err(());
        }
    }

    // is it an enum value
    if let Some(expr) = enum_value(
        loc,
        e,
        id,
        context.file_no,
        context.contract_no,
        ns,
        diagnostics,
    )? {
        return Ok(expr);
    }

    // is it an event selector
    if let Some(expr) = event_selector(
        loc,
        e,
        id,
        context.file_no,
        context.contract_no,
        ns,
        diagnostics,
    )? {
        return Ok(expr);
    }

    // is it a constant (unless basecontract is a local variable)
    if let Some(expr) =
        contract_constant(loc, e, id, ns, symtable, context, diagnostics, resolve_to)?
    {
        return Ok(expr);
    }

    // is it a basecontract.function.selector expression (unless basecontract is a local variable)
    if let pt::Expression::Variable(namespace) = e {
        if symtable.find(context, &namespace.name).is_none() {
            if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
                // find function with this name
                let mut name_matches = 0;
                let mut expr = Err(());

                for function_no in ns.contracts[call_contract_no].all_functions.keys() {
                    let func = &ns.functions[*function_no];

                    if func.id.name != id.name || func.ty != pt::FunctionTy::Function {
                        continue;
                    }

                    name_matches += 1;

                    let mut id_path = ns.expr_to_identifier_path(e).unwrap();
                    id_path.identifiers.push(id.clone());
                    id_path.loc = *loc;

                    expr = Ok(Expression::InternalFunction {
                        loc: e.loc(),
                        id: id_path,
                        ty: function_type(func, false, resolve_to),
                        function_no: *function_no,
                        signature: None,
                    })
                }

                return match name_matches {
                    0 => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            format!(
                                "contract '{}' does not have a member called '{}'",
                                ns.contracts[call_contract_no].id, id.name,
                            ),
                        ));
                        Err(())
                    }
                    1 => expr,
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            format!(
                                "function '{}' of contract '{}' is overloaded",
                                id.name, ns.contracts[call_contract_no].id,
                            ),
                        ));
                        Err(())
                    }
                };
            }
        }
    }

    let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

    if let Expression::TypeOperator { .. } = &expr {
        return type_name_expr(loc, expr, id, context, ns, diagnostics);
    }

    let expr_ty = expr.ty();

    if let Type::Struct(struct_ty) = expr_ty.deref_memory() {
        if let Some((i, f)) = struct_ty
            .definition(ns)
            .fields
            .iter()
            .enumerate()
            .find(|f| id.name == f.1.name_as_str())
        {
            return if context.lvalue && f.readonly {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "struct '{}' field '{}' is readonly",
                        struct_ty.definition(ns),
                        id.name
                    ),
                ));
                Err(())
            } else if f.readonly {
                // readonly fields return the value, not a reference
                Ok(Expression::StructMember {
                    loc: id.loc,
                    ty: f.ty.clone(),
                    expr: Box::new(expr),
                    field: i,
                })
            } else {
                Ok(Expression::StructMember {
                    loc: id.loc,
                    ty: Type::Ref(Box::new(f.ty.clone())),
                    expr: Box::new(expr),
                    field: i,
                })
            };
        } else {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!(
                    "struct '{}' does not have a field called '{}'",
                    struct_ty.definition(ns),
                    id.name
                ),
            ));
            return Err(());
        }
    }

    // Dereference if need to
    let (expr, expr_ty) = if let Type::Ref(ty) = &expr_ty {
        (
            Expression::Load {
                loc: *loc,
                ty: *ty.clone(),
                expr: Box::new(expr),
            },
            ty.as_ref().clone(),
        )
    } else {
        (expr, expr_ty)
    };

    match expr_ty {
        Type::Bytes(n) => {
            if id.name == "length" {
                //We should not eliminate an array from the code when 'length' is called
                //So the variable is also assigned a value to be read from 'length'
                assigned_variable(ns, &expr, symtable);
                used_variable(ns, &expr, symtable);
                return Ok(Expression::NumberLiteral {
                    loc: *loc,
                    ty: Type::Uint(8),
                    value: BigInt::from_u8(n).unwrap(),
                });
            }
        }
        Type::Array(elem_ty, dim) => {
            if id.name == "length" {
                return match dim.last().unwrap() {
                    ArrayLength::Dynamic => Ok(Expression::Builtin {
                        loc: *loc,
                        tys: vec![Type::Uint(32)],
                        kind: Builtin::ArrayLength,
                        args: vec![expr],
                    }),
                    ArrayLength::Fixed(d) => {
                        //We should not eliminate an array from the code when 'length' is called
                        //So the variable is also assigned a value to be read from 'length'
                        assigned_variable(ns, &expr, symtable);
                        used_variable(ns, &expr, symtable);
                        bigint_to_expression(
                            loc,
                            d,
                            ns,
                            diagnostics,
                            ResolveTo::Type(&Type::Uint(32)),
                            None,
                        )
                    }
                    ArrayLength::AnyFixed => unreachable!(),
                };
            } else if matches!(*elem_ty, Type::Struct(StructType::AccountInfo))
                && context.function_no.is_some()
                && ns.target == Target::Solana
            {
                return if ns.functions[context.function_no.unwrap()]
                    .solana_accounts
                    .borrow()
                    .contains_key(&id.name)
                    || id.name == BuiltinAccounts::DataAccount
                {
                    Ok(Expression::NamedMember {
                        loc: *loc,
                        ty: Type::Ref(Box::new(Type::Struct(StructType::AccountInfo))),
                        array: Box::new(expr),
                        name: id.name.clone(),
                    })
                } else {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        "unrecognized account".to_string(),
                    ));
                    Err(())
                };
            }
        }
        Type::String | Type::DynamicBytes | Type::Slice(_) => {
            if id.name == "length" {
                return Ok(Expression::Builtin {
                    loc: *loc,
                    tys: vec![Type::Uint(32)],
                    kind: Builtin::ArrayLength,
                    args: vec![expr],
                });
            }
        }
        Type::StorageRef(immutable, r) => match *r {
            Type::Struct(str_ty) => {
                return if let Some((field_no, field)) = str_ty
                    .definition(ns)
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| id.name == field.name_as_str())
                {
                    Ok(Expression::StructMember {
                        loc: id.loc,
                        ty: Type::StorageRef(immutable, Box::new(field.ty.clone())),
                        expr: Box::new(expr),
                        field: field_no,
                    })
                } else {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "struct '{}' does not have a field called '{}'",
                            str_ty.definition(ns).id,
                            id.name
                        ),
                    ));
                    Err(())
                }
            }
            Type::Array(_, dim) => {
                if id.name == "length" {
                    let elem_ty = expr.ty().storage_array_elem().deref_into();

                    if let Some(ArrayLength::Fixed(dim)) = dim.last() {
                        // sparse array could be large than ns.storage_type() on Solana
                        if dim.bits() > ns.storage_type().bits(ns) as u64 {
                            return Ok(Expression::StorageArrayLength {
                                loc: id.loc,
                                ty: Type::Uint(256),
                                array: Box::new(expr),
                                elem_ty,
                            });
                        }
                    }

                    return Ok(Expression::StorageArrayLength {
                        loc: id.loc,
                        ty: ns.storage_type(),
                        array: Box::new(expr),
                        elem_ty,
                    });
                }
            }
            Type::Bytes(_) | Type::DynamicBytes | Type::String => {
                if id.name == "length" {
                    let elem_ty = expr.ty().storage_array_elem().deref_into();

                    return Ok(Expression::StorageArrayLength {
                        loc: id.loc,
                        ty: Type::Uint(32),
                        array: Box::new(expr),
                        elem_ty,
                    });
                }
            }
            _ => {}
        },
        Type::Address(_) if id.name == "balance" => {
            if ns.target.is_polkadot() {
                let mut is_this = false;

                if let Expression::Cast { expr: this, .. } = &expr {
                    if let Expression::Builtin {
                        kind: Builtin::GetAddress,
                        ..
                    } = this.as_ref()
                    {
                        is_this = true;
                    }
                }

                if !is_this {
                    diagnostics.push(Diagnostic::error(
                        expr.loc(),
                        "polkadot can only retrieve balance of 'this', like 'address(this).balance'"
                            .to_string(),
                    ));
                    return Err(());
                }
            } else if ns.target == Target::Solana {
                diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    "balance is not available on Solana. Use \
                    tx.accounts.account_name.lamports to fetch the balance."
                        .to_string(),
                ));
                return Err(());
            }
            used_variable(ns, &expr, symtable);
            return Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![Type::Value],
                kind: Builtin::Balance,
                args: vec![expr],
            });
        }
        Type::Address(_) if id.name == "code" => {
            if ns.target != Target::EVM {
                diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    format!("'address.code' is not supported on {}", ns.target),
                ));
                return Err(());
            }
            used_variable(ns, &expr, symtable);
            return Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![Type::DynamicBytes],
                kind: Builtin::ContractCode,
                args: vec![expr],
            });
        }
        Type::Contract(ref_contract_no) => {
            let mut name_matches = 0;
            let mut ext_expr = Err(());

            for function_no in ns.contracts[ref_contract_no].all_functions.keys() {
                let func = &ns.functions[*function_no];

                if func.id.name != id.name
                    || func.ty != pt::FunctionTy::Function
                    || !func.is_public()
                {
                    continue;
                }

                let ty = Type::ExternalFunction {
                    params: func.params.iter().map(|p| p.ty.clone()).collect(),
                    mutability: func.mutability.clone(),
                    returns: func.returns.iter().map(|p| p.ty.clone()).collect(),
                };

                name_matches += 1;
                ext_expr = Ok(Expression::ExternalFunction {
                    loc: id.loc,
                    ty,
                    address: Box::new(expr.clone()),
                    function_no: *function_no,
                });
            }

            return match name_matches {
                0 => {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "{} '{}' has no public function '{}'",
                            ns.contracts[ref_contract_no].ty,
                            ns.contracts[ref_contract_no].id,
                            id.name
                        ),
                    ));
                    Err(())
                }
                1 => ext_expr,
                _ => {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "function '{}' of {} '{}' is overloaded",
                            id.name,
                            ns.contracts[ref_contract_no].ty,
                            ns.contracts[ref_contract_no].id
                        ),
                    ));
                    Err(())
                }
            };
        }
        Type::ExternalFunction { .. } => {
            if id.name == "address" {
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin {
                    loc: e.loc(),
                    tys: vec![Type::Address(false)],
                    kind: Builtin::ExternalFunctionAddress,
                    args: vec![expr],
                });
            }
            if id.name == "selector" {
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin {
                    loc: e.loc(),
                    tys: vec![Type::FunctionSelector],
                    kind: Builtin::FunctionSelector,
                    args: vec![expr],
                });
            }
        }
        Type::InternalFunction { .. } => {
            if let Expression::InternalFunction { .. } = expr {
                if id.name == "selector" {
                    used_variable(ns, &expr, symtable);
                    return Ok(Expression::Builtin {
                        loc: e.loc(),
                        tys: vec![Type::FunctionSelector],
                        kind: Builtin::FunctionSelector,
                        args: vec![expr],
                    });
                }
            }
        }
        _ => (),
    }

    diagnostics.push(Diagnostic::error(*loc, format!("'{}' not found", id.name)));

    Err(())
}

fn contract_constant(
    loc: &pt::Loc,
    e: &pt::Expression,
    id: &pt::Identifier,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    context: &mut ExprContext,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    let namespace = match e {
        pt::Expression::Variable(namespace) => namespace,
        _ => return Ok(None),
    };

    if symtable.find(context, &namespace.name).is_some() {
        return Ok(None);
    }

    if let Some(contract_no) = ns.resolve_contract(context.file_no, namespace) {
        if let Some((var_no, var)) = ns.contracts[contract_no]
            .variables
            .iter_mut()
            .enumerate()
            .find(|(_, variable)| variable.name == id.name)
        {
            if !var.constant {
                let resolve_function = if let ResolveTo::Type(ty) = resolve_to {
                    matches!(
                        ty,
                        Type::InternalFunction { .. } | Type::ExternalFunction { .. }
                    )
                } else {
                    false
                };

                if resolve_function {
                    // requested function, fall through
                    return Ok(None);
                } else {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "need instance of contract '{}' to get variable value '{}'",
                            ns.contracts[contract_no].id,
                            ns.contracts[contract_no].variables[var_no].name,
                        ),
                    ));
                    return Err(());
                }
            }

            var.read = true;

            return Ok(Some(Expression::ConstantVariable {
                loc: *loc,
                ty: var.ty.clone(),
                contract_no: Some(contract_no),
                var_no,
            }));
        }
    }

    Ok(None)
}

/// Try to resolve expression as an enum value. An enum can be prefixed
/// with import symbols, contract namespace before the enum type
fn enum_value(
    loc: &pt::Loc,
    expr: &pt::Expression,
    id: &pt::Identifier,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Expression>, ()> {
    let mut namespace = Vec::new();

    let mut expr = expr;

    // the first element of the path is the deepest in the parse tree,
    // so walk down and add to a list
    while let pt::Expression::MemberAccess(_, member, name) = expr {
        namespace.push(name);

        expr = member.as_ref();
    }

    if let pt::Expression::Variable(name) = expr {
        namespace.push(name);
    } else {
        return Ok(None);
    }

    // The leading part of the namespace can be import variables
    let mut file_no = file_no;

    // last element in our namespace vector is first element
    while let Some(name) = namespace.last().map(|f| f.name.clone()) {
        if let Some(Symbol::Import(_, import_file_no)) =
            ns.variable_symbols.get(&(file_no, None, name))
        {
            file_no = *import_file_no;
            namespace.pop();
        } else {
            break;
        }
    }

    if namespace.is_empty() {
        return Ok(None);
    }

    let mut contract_no = contract_no;

    if let Some(no) = ns.resolve_contract(file_no, namespace.last().unwrap()) {
        contract_no = Some(no);
        namespace.pop();
    }

    if namespace.len() != 1 {
        return Ok(None);
    }

    if let Some(e) = ns.resolve_enum(file_no, contract_no, namespace[0]) {
        match ns.enums[e].values.get_full(&id.name) {
            Some((val, _, _)) => Ok(Some(Expression::NumberLiteral {
                loc: *loc,
                ty: Type::Enum(e),
                value: BigInt::from_usize(val).unwrap(),
            })),
            None => {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("enum {} does not have value {}", ns.enums[e], id.name),
                ));
                Err(())
            }
        }
    } else {
        Ok(None)
    }
}

fn event_selector(
    loc: &pt::Loc,
    expr: &pt::Expression,
    id: &pt::Identifier,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Expression>, ()> {
    if id.name != "selector" {
        return Ok(None);
    }

    if let Ok(events) = ns.resolve_event(file_no, contract_no, expr, &mut Diagnostics::default()) {
        if events.len() == 1 {
            let event_no = events[0];

            if ns.events[event_no].anonymous {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "anonymous event has no selector".into(),
                ));
                Err(())
            } else {
                Ok(Some(Expression::EventSelector {
                    loc: *loc,
                    event_no,
                    ty: if ns.target == Target::Solana {
                        Type::Bytes(8)
                    } else {
                        Type::Bytes(32)
                    },
                }))
            }
        } else {
            let notes = events
                .into_iter()
                .map(|ev_no| {
                    let ev = &ns.events[ev_no];
                    Note {
                        loc: ev.id.loc,
                        message: format!("possible definition of '{}'", ev.id),
                    }
                })
                .collect();

            diagnostics.push(Diagnostic::error_with_notes(
                *loc,
                "multiple definitions of event".into(),
                notes,
            ));
            Err(())
        }
    } else {
        Ok(None)
    }
}

/// Resolve type(x).foo
fn type_name_expr(
    loc: &pt::Loc,
    expr: Expression,
    field: &pt::Identifier,
    context: &mut ExprContext,
    ns: &mut Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let Expression::TypeOperator { ty, .. } = &expr else {
        unreachable!();
    };

    match field.name.as_str() {
        "min" | "max" if matches!(ty, Type::Uint(_) | Type::Int(_) | Type::Enum(..)) => {
            let ty = if matches!(ty, Type::Enum(..)) {
                Type::Uint(8)
            } else {
                ty.clone()
            };
            let kind = if field.name == "min" {
                Builtin::TypeMin
            } else {
                Builtin::TypeMax
            };

            return Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![ty],
                kind,
                args: vec![expr],
            });
        }
        "name" if matches!(ty, Type::Contract(..)) => {
            return Ok(Expression::Builtin {
                loc: *loc,
                tys: vec![Type::String],
                kind: Builtin::TypeName,
                args: vec![expr],
            })
        }
        "interfaceId" => {
            if let Type::Contract(no) = ty {
                let contract = &ns.contracts[*no];

                return if !contract.is_interface() {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "type(…).interfaceId is permitted on interface, not {} {}",
                            contract.ty, contract.id
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Builtin {
                        loc: *loc,
                        tys: vec![Type::FunctionSelector],
                        kind: Builtin::TypeInterfaceId,
                        args: vec![expr],
                    })
                };
            }
        }
        "creationCode" | "runtimeCode" => {
            if let Type::Contract(no) = ty {
                if !ns.contracts[*no].instantiable {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "cannot construct '{}' of type '{}'",
                            ns.contracts[*no].id, ns.contracts[*no].ty
                        ),
                    ));

                    return Err(());
                }

                // This is not always in a function: e.g. contract constant:
                // contract C {
                //      bytes constant code = type(D).runtimeCode;
                // }
                if let Some(function_no) = context.function_no {
                    ns.functions[function_no].creates.push((*loc, *no));
                }

                if let Some(contract_no) = context.contract_no {
                    // check for circular references
                    if *no == contract_no {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "cannot construct current contract '{}'",
                                ns.contracts[*no].id
                            ),
                        ));
                        return Err(());
                    }

                    if circular_reference(*no, contract_no, ns) {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "circular reference creating contract code for '{}'",
                                ns.contracts[*no].id
                            ),
                        ));
                        return Err(());
                    }

                    if !ns.contracts[contract_no].creates.contains(no) {
                        ns.contracts[contract_no].creates.push(*no);
                    }
                }

                let kind = if field.name == "runtimeCode" {
                    if ns.target == Target::EVM {
                        let notes: Vec<_> = ns.contracts[*no]
                            .variables
                            .iter()
                            .filter_map(|v| {
                                if v.immutable {
                                    Some(Note {
                                        loc: v.loc,
                                        message: format!("immutable variable {}", v.name),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !notes.is_empty() {
                            diagnostics.push(Diagnostic::error_with_notes(
                                *loc,
                                format!(
                                    "runtimeCode is not available for contract '{}' with immutuables",
                                    ns.contracts[*no].id
                                ),
                                notes,
                            ));
                        }
                    }

                    Builtin::TypeRuntimeCode
                } else {
                    Builtin::TypeCreatorCode
                };

                return Ok(Expression::Builtin {
                    loc: *loc,
                    tys: vec![Type::DynamicBytes],
                    kind,
                    args: vec![expr],
                });
            }
        }
        _ => (),
    };

    diagnostics.push(Diagnostic::error(
        *loc,
        format!(
            "type '{}' does not have type function {}",
            ty.to_string(ns),
            field.name
        ),
    ));
    Err(())
}
