
use ast::*;
use std::collections::HashMap;

pub fn resolve(s: &mut SourceUnit) {
    for p in &mut s.1 {
        if let SourceUnitPart::ContractDefinition(ref mut def) = p {
            if def.0 == ContractType::Contract {
                for m in &mut def.2 {
                    if let ContractPart::FunctionDefinition(ref mut func) = m {
                        resolve_func(func);
                    }
                }
            }
        }
    }
}

fn visit_statement(s: &Statement, f: &mut FnMut(&Statement)) {
    f(s);
    
    match s {
        Statement::BlockStatement(BlockStatement(bs)) => {
            for i in bs {
                visit_statement(&i, f);
            }
        },
        Statement::For(i, _, n, b) => {
            if let box Some(j) = i {
                visit_statement(&j, f);
            }
            if let box Some(j) = n {
                visit_statement(&j, f);
            }
            if let box Some(j) = b {
                visit_statement(&j, f);
            }
        },
        Statement::While(_, b) => {
            visit_statement(&b, f);
        },
        Statement::If(_, then, _else) => {
            visit_statement(&then, f);
            if let box Some(b) = _else {
                visit_statement(&b, f);
            }
        },
        _ => ()
    }
}

fn resolve_func(f: &mut Box<FunctionDefinition>) {
    // find all the variables
    let mut vartable = HashMap::new();

    visit_statement(&f.body, &mut |s| {
        if let Statement::VariableDefinition(v, _) = s {
            let name = &v.2;

            if vartable.contains_key(name) {
                println!("variable {} redeclared", name);
            } else {
                vartable.insert(name.to_string(), v.0);
            }
        }
    });

    f.vartable = Some(vartable);
}