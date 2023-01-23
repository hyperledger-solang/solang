// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{ArrayLength, Builtin, Expression, Namespace, RetrieveType, Symbol, Type};
use crate::sema::builtin;
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::constructor::circular_reference;
use crate::sema::expression::function_call::function_type;
use crate::sema::expression::integers::bigint_to_expression;
use crate::sema::expression::{expression, ExprContext, ResolveTo};
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::{assigned_variable, used_variable};
use num_bigint::{BigInt, Sign};
use num_traits::{FromPrimitive, One, Zero};
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;
use std::ops::{Shl, Sub};

/// Resolve an member access expression
pub(super) fn member_access(
    loc: &pt::Loc,
    e: &pt::Expression,
    id: &pt::Identifier,
    context: &ExprContext,
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

    // is it a constant (unless basecontract is a local variable)
    if let Some(expr) = contract_constant(
        loc,
        e,
        id,
        context.file_no,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )? {
        return Ok(expr);
    }

    // is it a basecontract.function.selector expression (unless basecontract is a local variable)
    if let pt::Expression::Variable(namespace) = e {
        if symtable.find(&namespace.name).is_none() {
            if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
                // find function with this name
                let mut name_matches = 0;
                let mut expr = Err(());

                for function_no in ns.contracts[call_contract_no].all_functions.keys() {
                    let func = &ns.functions[*function_no];

                    if func.name != id.name || func.ty != pt::FunctionTy::Function {
                        continue;
                    }

                    name_matches += 1;

                    expr = Ok(Expression::InternalFunction {
                        loc: e.loc(),
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
                                ns.contracts[call_contract_no].name, id.name,
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
                                id.name, ns.contracts[call_contract_no].name,
                            ),
                        ));
                        Err(())
                    }
                };
            }
        }
    }

    // is of the form "type(x).field", like type(c).min
    if let pt::Expression::FunctionCall(_, name, args) = e {
        if let pt::Expression::Variable(func_name) = name.as_ref() {
            if func_name.name == "type" {
                return type_name_expr(loc, args, id, context, ns, diagnostics, resolve_to);
            }
        }
    }

    let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;
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
                ty: expr_ty.clone(),
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
        Type::Array(_, dim) => {
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
                        )
                    }
                    ArrayLength::AnyFixed => unreachable!(),
                };
            }
        }
        Type::String | Type::DynamicBytes => {
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
                            str_ty.definition(ns).name,
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
        Type::Address(_) => {
            if id.name == "balance" {
                if ns.target.is_substrate() {
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
                            "substrate can only retrieve balance of this, like 'address(this).balance'".to_string(),
                        ));
                        return Err(());
                    }
                }
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin {
                    loc: *loc,
                    tys: vec![Type::Value],
                    kind: Builtin::Balance,
                    args: vec![expr],
                });
            }
        }
        Type::Contract(ref_contract_no) => {
            let mut name_matches = 0;
            let mut ext_expr = Err(());

            for function_no in ns.contracts[ref_contract_no].all_functions.keys() {
                let func = &ns.functions[*function_no];

                if func.name != id.name || func.ty != pt::FunctionTy::Function || !func.is_public()
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
                            ns.contracts[ref_contract_no].name,
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
                            ns.contracts[ref_contract_no].name
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
    file_no: usize,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    let namespace = match e {
        pt::Expression::Variable(namespace) => namespace,
        _ => return Ok(None),
    };

    if symtable.find(&namespace.name).is_some() {
        return Ok(None);
    }

    if let Some(contract_no) = ns.resolve_contract(file_no, namespace) {
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
                            ns.contracts[contract_no].name,
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

/// Resolve type(x).foo
fn type_name_expr(
    loc: &pt::Loc,
    args: &[pt::Expression],
    field: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if args.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "missing argument to type()".to_string(),
        ));
        return Err(());
    }

    if args.len() > 1 {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!("got {} arguments to type(), only one expected", args.len(),),
        ));
        return Err(());
    }

    let ty = ns.resolve_type(
        context.file_no,
        context.contract_no,
        false,
        &args[0],
        diagnostics,
    )?;

    match (&ty, field.name.as_str()) {
        (Type::Uint(_), "min") => {
            bigint_to_expression(loc, &BigInt::zero(), ns, diagnostics, resolve_to)
        }
        (Type::Uint(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize).sub(1);
            bigint_to_expression(loc, &max, ns, diagnostics, resolve_to)
        }
        (Type::Int(bits), "min") => {
            let min = BigInt::zero().sub(BigInt::one().shl(*bits as usize - 1));
            bigint_to_expression(loc, &min, ns, diagnostics, resolve_to)
        }
        (Type::Int(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize - 1).sub(1);
            bigint_to_expression(loc, &max, ns, diagnostics, resolve_to)
        }
        (Type::Contract(n), "name") => Ok(Expression::BytesLiteral {
            loc: *loc,
            ty: Type::String,
            value: ns.contracts[*n].name.as_bytes().to_vec(),
        }),
        (Type::Contract(n), "interfaceId") => {
            let contract = &ns.contracts[*n];

            if !contract.is_interface() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "type(…).interfaceId is permitted on interface, not {} {}",
                        contract.ty, contract.name
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::InterfaceId {
                    loc: *loc,
                    contract_no: *n,
                })
            }
        }
        (Type::Contract(no), "program_id") => {
            let contract = &ns.contracts[*no];

            if let Some(v) = &contract.program_id {
                Ok(Expression::NumberLiteral {
                    loc: *loc,
                    ty: Type::Address(false),
                    value: BigInt::from_bytes_be(Sign::Plus, v),
                })
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "{} '{}' has no declared program_id",
                        contract.ty, contract.name
                    ),
                ));
                Err(())
            }
        }
        (Type::Contract(no), "creationCode") | (Type::Contract(no), "runtimeCode") => {
            let contract_no = match context.contract_no {
                Some(contract_no) => contract_no,
                None => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "type().{} not permitted outside of contract code",
                            field.name
                        ),
                    ));
                    return Err(());
                }
            };

            // check for circular references
            if *no == contract_no {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "containing our own contract code for '{}' would generate infinite size contract",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if circular_reference(*no, contract_no, ns) {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "circular reference creating contract code for '{}'",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if !ns.contracts[contract_no].creates.contains(no) {
                ns.contracts[contract_no].creates.push(*no);
            }

            Ok(Expression::CodeLiteral {
                loc: *loc,
                contract_no: *no,
                runtime: field.name == "runtimeCode",
            })
        }
        _ => {
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
    }
}
