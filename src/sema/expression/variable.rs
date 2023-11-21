// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Builtin, Expression, Namespace, Symbol, Type};
use crate::sema::builtin;
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::function_call::available_functions;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::symtable::Symtable;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;

pub(super) fn variable(
    id: &pt::Identifier,
    context: &mut ExprContext,
    ns: &Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let Some(v) = symtable.find(context, &id.name) {
        return if context.constant {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!("cannot read variable '{}' in constant expression", id.name),
            ));
            Err(())
        } else {
            Ok(Expression::Variable {
                loc: id.loc,
                ty: v.ty.clone(),
                var_no: v.pos,
            })
        };
    }

    if let Some((builtin, ty)) = builtin::builtin_var(&id.loc, None, &id.name, ns, diagnostics) {
        return Ok(Expression::Builtin {
            loc: id.loc,
            tys: vec![ty],
            kind: builtin,
            args: vec![],
        });
    }

    // are we trying to resolve a function type?
    let function_first = if let ResolveTo::Type(resolve_to) = resolve_to {
        matches!(
            resolve_to,
            Type::InternalFunction { .. } | Type::ExternalFunction { .. }
        )
    } else {
        false
    };

    match ns.resolve_var(context.file_no, context.contract_no, id, function_first) {
        Some(Symbol::Variable(_, Some(var_contract_no), var_no)) => {
            let var_contract_no = *var_contract_no;
            let var_no = *var_no;

            let var = &ns.contracts[var_contract_no].variables[var_no];

            if var.constant {
                Ok(Expression::ConstantVariable {
                    loc: id.loc,
                    ty: var.ty.clone(),
                    contract_no: Some(var_contract_no),
                    var_no,
                })
            } else if context.constant {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "cannot read contract variable '{}' in constant expression",
                        id.name
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::StorageVariable {
                    loc: id.loc,
                    ty: Type::StorageRef(var.immutable, Box::new(var.ty.clone())),
                    contract_no: var_contract_no,
                    var_no,
                })
            }
        }
        Some(Symbol::Variable(_, None, var_no)) => {
            let var_no = *var_no;

            let var = &ns.constants[var_no];

            Ok(Expression::ConstantVariable {
                loc: id.loc,
                ty: var.ty.clone(),
                contract_no: None,
                var_no,
            })
        }
        Some(Symbol::Function(_)) => {
            let mut name_matches = 0;
            let mut expr = None;

            for function_no in
                available_functions(&id.name, true, context.file_no, context.contract_no, ns)
            {
                let func = &ns.functions[function_no];

                if func.ty != pt::FunctionTy::Function {
                    continue;
                }

                let ty = Type::InternalFunction {
                    params: func.params.iter().map(|p| p.ty.clone()).collect(),
                    mutability: func.mutability.clone(),
                    returns: func.returns.iter().map(|p| p.ty.clone()).collect(),
                };

                name_matches += 1;

                let id_path = pt::IdentifierPath {
                    loc: id.loc,
                    identifiers: vec![id.clone()],
                };

                expr = Some(Expression::InternalFunction {
                    loc: id.loc,
                    id: id_path,
                    ty,
                    function_no,
                    signature: if func.is_virtual || func.is_override.is_some() {
                        Some(func.signature.clone())
                    } else {
                        None
                    },
                });
            }

            if name_matches == 1 {
                Ok(expr.unwrap())
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("function '{}' is overloaded", id.name),
                ));
                Err(())
            }
        }
        None if id.name == "now"
            && matches!(
                resolve_to,
                ResolveTo::Type(Type::Uint(_)) | ResolveTo::Integer
            ) =>
        {
            diagnostics.push(
                    Diagnostic::error(
                        id.loc,
                        "'now' not found. 'now' was an alias for 'block.timestamp' in older versions of the Solidity language. Please use 'block.timestamp' instead.".to_string(),
                    ));
            Err(())
        }
        None if id.name == "this" => match context.contract_no {
            Some(contract_no) => Ok(Expression::Builtin {
                loc: id.loc,
                tys: vec![Type::Contract(contract_no)],
                kind: Builtin::GetAddress,
                args: Vec::new(),
            }),
            None => {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    "this not allowed outside contract".to_owned(),
                ));
                Err(())
            }
        },
        sym => {
            diagnostics.push(Namespace::wrong_symbol(sym, id));
            Err(())
        }
    }
}
