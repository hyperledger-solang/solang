
use ast::*;
use std::cmp;
use std::collections::HashMap;
use num_bigint::Sign;

pub fn resolve(s: &mut SourceUnit) -> Result<(), String> {
    for p in &mut s.1 {
        if let SourceUnitPart::ContractDefinition(ref mut def) = p {
            if def.0 == ContractType::Contract {
                for m in &mut def.2 {
                    if let ContractPart::FunctionDefinition(ref mut func) = m {
                        resolve_func(func)?;
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn visit_statement(s: &Statement, f: &mut FnMut(&Statement) -> Result<(), String>) -> Result<(), String> {
    f(s)?;
    
    match s {
        Statement::BlockStatement(BlockStatement(bs)) => {
            for i in bs {
                visit_statement(&i, f)?;
            }
        },
        Statement::For(i, _, n, b) => {
            if let box Some(j) = i {
                visit_statement(&j, f)?;
            }
            if let box Some(j) = n {
                visit_statement(&j, f)?;
            }
            if let box Some(j) = b {
                visit_statement(&j, f)?;
            }
        },
        Statement::While(_, b) => {
            visit_statement(&b, f)?;
        },
        Statement::If(_, then, _else) => {
            visit_statement(&then, f)?;
            if let box Some(b) = _else {
                visit_statement(&b, f)?;
            }
        },
        _ => ()
    }

    Ok(())
}

fn resolve_func(f: &mut Box<FunctionDefinition>) -> Result<(), String> {
    // find all the variables
    let mut vartable = HashMap::new();

    for p in &f.params {
        if let Some(ref n) = p.2 {
            vartable.insert(n.to_string(), p.0);
        }
    }

    visit_statement(&f.body, &mut |s| {
        if let Statement::VariableDefinition(v, _) = s {
            let name = &v.2;

            if vartable.contains_key(name) {
                return Err(format!("variable {} redeclared", name));
            } else {
                vartable.insert(name.to_string(), v.0);
            }
        }
        Ok(())
    })?;

    f.vartable = Some(vartable);

    // Check expressions
    visit_statement(&f.body, &mut |s| {
        match s {
            Statement::VariableDefinition(decl, Some(expr)) => {
                check_expression(f, expr, decl.0)
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
                    check_expression(f, expr, f.returns[0].0)
                } else {
                    Ok(())
                }
            },
            Statement::BlockStatement(_) => Ok(()),
            _ => Err(format!("resolve of statement {:?} not implement yet", s))
        }
    })?;

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
        Expression::Variable(s) => {
            if let Some(ref vartable) = f.vartable {
                match vartable.get(s) {
                    Some(v) => Ok(*v),
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
        Expression::Add(l, r) => binary_expression(f, l, r),
        Expression::Subtract(l, r) => binary_expression(f, l, r),
        Expression::Multiply(l, r) => binary_expression(f, l, r),
        Expression::Modulo(l, r) => binary_expression(f, l, r),
        Expression::Assign(l, r) |
        Expression::AssignAdd(l, r) |
        Expression::AssignSubtract(l, r) => binary_expression(f, l, r),
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