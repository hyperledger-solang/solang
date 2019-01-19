
use ast::*;
use std::cmp;
use std::collections::HashMap;
use num_bigint::Sign;
use output::Output;

pub fn resolve(s: &mut SourceUnit) -> Vec<Output> {
    let mut errors = Vec::new();

    for p in &mut s.parts {
        if let SourceUnitPart::ContractDefinition(ref mut def) = p {
            if def.typ == ContractType::Contract {
                for m in &mut def.parts {
                    if let ContractPart::FunctionDefinition(ref mut func) = m {
                        let _ = resolve_func(func, &mut errors);
                    }
                }
            }
        }
    }

    let mut fatal = 0;

    for e in &errors {
        if e.is_fatal() {
            fatal += 1;
        }
    }

    s.resolved = fatal == 0;

    errors
}

fn resolve_func(f: &mut Box<FunctionDefinition>, error: &mut Vec<Output>) -> Result<(), ()> {
    // find all the variables
    let mut vartable = HashMap::new();

    for p in &f.params {
        if let Some(ref n) = p.name {
            vartable.insert(n.name.to_string(), p.typ);
        }
    }

    for r in &f.returns {
        if let Some(ref name) = r.name {
            error.push(Output::warning(name.loc.clone(), format!("named return value `{}' not allowed", name.name)));
        }
    }

    f.body.visit_stmt(&mut |s| {
        if let Statement::VariableDefinition(v, _) = s {
            let name = &v.name;

            if vartable.contains_key(&name.name) {
                error.push(Output::error(name.loc.clone(), format!("variable {} redeclared", name.name)));
            } else {
                vartable.insert(name.name.to_string(), v.typ);
            }
        }
        Ok(())
    }).expect("should succeed");

    f.vartable = Some(vartable);

    // Check expressions
    f.body.visit_stmt(&mut |s| {
        match s {
            Statement::VariableDefinition(decl, Some(expr)) => {
                check_expression(f, expr, decl.typ, error)
            },
            Statement::VariableDefinition(_, None) => {
                Ok(())
            },
            Statement::Expression(expr) => {
                get_expression_type(f, expr, error)?;

                Ok(())
            }
            Statement::If(expr, _, _) => {
                check_expression(f, expr, ElementaryTypeName::Bool, error)
            },
            Statement::For(_, expr, _, _) => {
                if let box Some(expr) = expr {
                    check_expression(f, expr, ElementaryTypeName::Bool, error)?;
                }
                Ok(())
            },
            Statement::While(expr, _) => {
                check_expression(f, expr, ElementaryTypeName::Bool, error)
            },
            Statement::DoWhile(_, expr) => {
                check_expression(f, expr, ElementaryTypeName::Bool, error)
            },
            Statement::Return(_, None) => {
                // actually this allowed if all return values have names
                if f.returns.len() > 0 {
                    error.push(Output::error(Loc(0, 0), format!("missing return value, {} expected", f.returns.len())));
                }
                Ok(())
            },
            Statement::Return(_, Some(expr)) => {
                if f.returns.len() == 0 {
                    error.push(Output::error(Loc(0, 0), format!("this function has no return value")));
                } else if f.returns.len() == 1 {
                    check_expression(f, expr, f.returns[0].typ, error)?;
                }

                Ok(())
            },
            Statement::BlockStatement(_) => Ok(()),
            Statement::Break => Ok(()),
            _ => panic!(format!("resolve of statement {:?} not implement yet", s))
        }
    })?;

    // check for unreachable code (anything after return,break,continue)
    // check for infinite loops
    // check if function ends with return

    Ok(())
}

pub fn coercion_type(left: ElementaryTypeName, right: ElementaryTypeName) -> Option<ElementaryTypeName> {
    if left == right {
        return Some(left);
    }

    if let ElementaryTypeName::Int(l) = left {
        return match right {
            ElementaryTypeName::Int(r) => {
                Some(ElementaryTypeName::Int(cmp::max(l, r)))
            },
            ElementaryTypeName::Uint(r) if r <= 255 => {
                Some(ElementaryTypeName::Int(cmp::max(l, r+1)))
            },
            _ => None
        };
    }

    if let ElementaryTypeName::Uint(l) = left {
        return match right {
            ElementaryTypeName::Int(r) if l < 255 => {
                Some(ElementaryTypeName::Int(cmp::max(l+1, r)))
            },
            ElementaryTypeName::Uint(r) => {
                Some(ElementaryTypeName::Int(cmp::max(l, r)))
            },
            _ => None
        };
    }

    None
}

fn binary_expression(f: &FunctionDefinition, l: &Expression, r: &Expression, loc: &Loc, errors: &mut Vec<Output>) -> Result<ElementaryTypeName, ()> {
    let left = get_expression_type(f, l, errors)?;
    let right = get_expression_type(f, r, errors)?;

    if let Some(v) = coercion_type(left, right) {
        Ok(v)
    } else {
        errors.push(Output::error(loc.clone(), format!("cannot convert {} to {}", left.to_string(), right.to_string())));
        Err(())
    }
}

pub fn get_expression_type(f: &FunctionDefinition, e: &Expression, errors: &mut Vec<Output>) -> Result<ElementaryTypeName, ()> {
    match e {
        Expression::BoolLiteral(_, _) => Ok(ElementaryTypeName::Bool),
        Expression::StringLiteral(_, _) => Ok(ElementaryTypeName::String),
        Expression::NumberLiteral(loc, b) => {
            // Return smallest type
            let mut bits = b.bits();

            if bits < 7 {
                bits = 8;
            } else {
                bits = (bits + 7) & !7;
            }

            if b.sign() == Sign::Minus {
                if bits > 255 {
                    errors.push(Output::error(loc.clone(), format!("{} is too large", b)));
                    Err(())
                } else {
                    Ok(ElementaryTypeName::Int(bits as u16))
                }
            } else {
                if bits > 256 {
                    errors.push(Output::error(loc.clone(), format!("{} is too large", b)));
                    Err(())
                } else {
                    Ok(ElementaryTypeName::Uint(bits as u16))
                }
            }
        },
        Expression::Variable(t, s) => {
            if let Some(ref vartable) = f.vartable {
                match vartable.get(&s.name) {
                    Some(v) => {
                        t.set(*v);
                        Ok(*v)
                    }
                    ,
                    None => {
                        errors.push(Output::error(s.loc.clone(), format!("variable {} not found", s.name)));
                        Err(())
                    }
                }
            } else {
                panic!("vartable not there");
            }
        },
        Expression::PostDecrement(_, box Expression::Variable(t, s)) |
        Expression::PostIncrement(_, box Expression::Variable(t, s)) |
        Expression::PreDecrement(_, box Expression::Variable(t, s)) |
        Expression::PreIncrement(_, box Expression::Variable(t, s)) => {
            if let Some(ref vartable) = f.vartable {
                match vartable.get(&s.name) {
                    Some(v) => {
                        if !v.ordered() {
                            errors.push(Output::error(s.loc.clone(), format!("variable {} not a number", s.name)));
                            Err(())
                        } else {
                            t.set(*v);
                            Ok(*v)
                        }
                    }
                    ,
                    None => {
                        errors.push(Output::error(s.loc.clone(), format!("variable {} not found", s.name)));
                        Err(())
                    }
                }
            } else {
                panic!("vartable not there");
            }
        },
        Expression::Complement(_, e) => get_expression_type(f, e, errors),
        Expression::Not(_, e) => get_expression_type(f, e, errors),
        Expression::UnaryMinus(_, e) => get_expression_type(f, e, errors),
        Expression::UnaryPlus(_, e) => get_expression_type(f, e, errors),
        Expression::Add(loc, l, r) |
        Expression::Subtract(loc, l, r) |
        Expression::Multiply(loc, l, r) |
        Expression::Divide(loc, l, r) |
        Expression::Modulo(loc, l, r) => binary_expression(f, l, r, loc, errors),
        Expression::Assign(loc, l, r) |
        Expression::AssignMultiply(loc, l, r) |
        Expression::AssignDivide(loc, l, r) |
        Expression::AssignAdd(loc, l, r) |
        Expression::AssignSubtract(loc, l, r) => binary_expression(f, l, r, loc, errors),
        Expression::Equal(loc, l, r) => {
            binary_expression(f, l, r, loc, errors)?;
            Ok(ElementaryTypeName::Bool)
        },
        Expression::More(loc, l, r) |
        Expression::Less(loc, l, r) |
        Expression::MoreEqual(loc, l, r) |
        Expression::LessEqual(loc, l, r) => {
            let left = get_expression_type(f, l, errors)?;
            let right = get_expression_type(f, r, errors)?;

            if !left.ordered() {
                errors.push(Output::error(l.loc().clone(), format!("{} cannot be used in ordered compare", left.to_string())));
                return Err(());
            }

            if !right.ordered() {
                errors.push(Output::error(r.loc().clone(), format!("{} cannot be used in ordered compare", right.to_string())));
                return Err(());
            }

            if let Some(_) = coercion_type(left, right) {
                Ok(ElementaryTypeName::Bool)
            } else {
                errors.push(Output::error(loc.clone(), format!("cannot compare {} to {}", left.to_string(), right.to_string())));
                Err(())
            }
        },
        _ => panic!("resolve of expression {:?} not implemented yet", e)
    }
}

fn check_expression(f: &FunctionDefinition, e: &Expression, t: ElementaryTypeName, error: &mut Vec<Output>) -> Result<(), ()> {
    let etype = get_expression_type(f, e, error)?;

    if let None = coercion_type(etype, t) {
        error.push(Output::error(e.loc(), format!("cannot convert {} to {}", etype.to_string(), t.to_string())));
        Err(())
    } else {
        Ok(())
    }
}