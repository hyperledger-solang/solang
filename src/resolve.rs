
use ast::*;
use std::cmp;
use std::collections::HashMap;
use num_bigint::Sign;

pub fn resolve(s: &mut SourceUnit) -> Result<(), String> {
    for p in &mut s.parts {
        if let SourceUnitPart::ContractDefinition(ref mut def) = p {
            if def.typ == ContractType::Contract {
                for m in &mut def.parts {
                    if let ContractPart::FunctionDefinition(ref mut func) = m {
                        resolve_func(func)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn resolve_func(f: &mut Box<FunctionDefinition>) -> Result<(), String> {
    // find all the variables
    let mut vartable = HashMap::new();

    for p in &f.params {
        if let Some(ref n) = p.name {
            vartable.insert(n.to_string(), p.typ);
        }
    }

    for r in &f.returns {
        if let Some(_) = r.name {
            return Err(format!("named return values not allowed"));
        }
    }

    f.body.visit_stmt(&mut |s| {
        if let Statement::VariableDefinition(v, _) = s {
            let name = &v.name;

            if vartable.contains_key(name) {
                return Err(format!("variable {} redeclared", name));
            } else {
                vartable.insert(name.to_string(), v.typ);
            }
        }
        Ok(())
    })?;

    f.vartable = Some(vartable);

    // Check expressions
    f.body.visit_stmt(&mut |s| {
        match s {
            Statement::VariableDefinition(decl, Some(expr)) => {
                check_expression(f, expr, decl.typ)
            },
            Statement::VariableDefinition(_, None) => {
                Ok(())
            },
            Statement::Expression(expr) => {
                match get_expression_type(f, expr) {
                    Ok(_) => Ok(()),
                    Err(s) => Err(s)
                }
            }
            Statement::If(expr, _, _) => {
                check_expression(f, expr, ElementaryTypeName::Bool)
            },
            Statement::For(_, expr, _, _) => {
                if let box Some(expr) = expr {
                    check_expression(f, expr, ElementaryTypeName::Bool)
                } else {
                    Ok(())
                }
            },
            Statement::While(expr, _) => {
                check_expression(f, expr, ElementaryTypeName::Bool)
            },
            Statement::DoWhile(_, expr) => {
                check_expression(f, expr, ElementaryTypeName::Bool)
            },
            Statement::Return(None) => {
                // actually this allowed if all return values have names
                if f.returns.len() > 0 {
                    Err(format!("missing return value, {} expected", f.params.len()))
                } else {
                    Ok(())
                }
            },
            Statement::Return(Some(expr)) => {
                if f.returns.len() == 0 {
                    Err(format!("this function has no return value"))
                } else if f.returns.len() == 1 {
                    check_expression(f, expr, f.returns[0].typ)
                } else {
                    Ok(())
                }
            },
            Statement::BlockStatement(_) => Ok(()),
            _ => Err(format!("resolve of statement {:?} not implement yet", s))
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

fn binary_expression(f: &FunctionDefinition, l: &Expression, r: &Expression) -> Result<ElementaryTypeName, String> {
    let left = get_expression_type(f, l)?;
    let right = get_expression_type(f, r)?;

    if let Some(v) = coercion_type(left, right) {
        Ok(v)
    } else {
        Err(format!("cannot convert {:?} to {:?}", left, right))
    }
}

pub fn get_expression_type(f: &FunctionDefinition, e: &Expression) -> Result<ElementaryTypeName, String> {
    match e {
        Expression::BoolLiteral(_) => Ok(ElementaryTypeName::Bool),
        Expression::StringLiteral(_) => Ok(ElementaryTypeName::String),
        Expression::NumberLiteral(b) => {
            // Return smallest type
            let bits = b.bits();

            if b.sign() == Sign::Minus {
                if bits > 255 {
                    Err(format!("{} is too large", b))
                } else {
                    Ok(ElementaryTypeName::Int(bits as u16))
                }
            } else {
                if bits > 256 {
                    Err(format!("{} is too large", b))
                } else {
                    Ok(ElementaryTypeName::Uint(bits as u16))
                }
            }
        },
        Expression::Variable(t, s) => {
            if let Some(ref vartable) = f.vartable {
                match vartable.get(s) {
                    Some(v) => {
                        t.set(*v);
                        Ok(*v)
                    }
                    ,
                    None => Err(format!("variable {} not found", s))
                }
            } else {
                panic!("vartable not there");
            }
        },
        Expression::PostDecrement(box Expression::Variable(t, s)) |
        Expression::PostIncrement(box Expression::Variable(t, s)) |
        Expression::PreDecrement(box Expression::Variable(t, s)) |
        Expression::PreIncrement(box Expression::Variable(t, s)) => {
            if let Some(ref vartable) = f.vartable {
                match vartable.get(s) {
                    Some(v) => {
                        if !v.ordered() {
                            Err(format!("variable {} not a number", s))
                        } else {
                            t.set(*v);
                            Ok(*v)
                        }
                    }
                    ,
                    None => Err(format!("variable {} not found", s))
                }
            } else {
                panic!("vartable not there");
            }
        },
        Expression::Complement(e) => get_expression_type(f, e),
        Expression::Not(e) => get_expression_type(f, e),
        Expression::UnaryMinus(e) => get_expression_type(f, e),
        Expression::UnaryPlus(e) => get_expression_type(f, e),
        Expression::Add(l, r) |
        Expression::Subtract(l, r) |
        Expression::Multiply(l, r) |
        Expression::Divide(l, r) |
        Expression::Modulo(l, r) => binary_expression(f, l, r),
        Expression::Assign(l, r) |
        Expression::AssignMultiply(l, r) |
        Expression::AssignDivide(l, r) |
        Expression::AssignAdd(l, r) |
        Expression::AssignSubtract(l, r) => binary_expression(f, l, r),
        Expression::Equal(l, r) => {
            binary_expression(f, l, r)?;
            Ok(ElementaryTypeName::Bool)
        },
        Expression::More(l, r) |
        Expression::Less(l, r) |
        Expression::MoreEqual(l, r) |
        Expression::LessEqual(l, r) => {
            if !binary_expression(f, l, r)?.ordered() {
                return Err(format!("{:?} is not allowed", e));
            }

            Ok(ElementaryTypeName::Bool)
        }
        _ => Err(format!("resolve of expression {:?} not implemented yet", e))
    }
}

fn check_expression(f: &FunctionDefinition, e: &Expression, t: ElementaryTypeName) -> Result<(), String> {
    let etype = get_expression_type(f, e)?;

    if let None = coercion_type(etype, t) {
        Err(format!("cannot convert {:?} to {:?}", etype, t))
    } else {
        Ok(())
    }
}