
use parser::ast;
use super::cfg::{ControlFlowGraph, Vartable, Instr, Expression};
use resolver;

use super::{Contract, FunctionDecl, Parameter};

pub fn add_builtin_function(ns: &mut Contract) {
    let argty = resolver::Type::Primitive(ast::PrimitiveType::Bool);
    let id = ast::Identifier{
        loc: ast::Loc(0, 0),
        name: "assert".to_owned()
    };

    let mut assert = FunctionDecl::new(ast::Loc(0, 0), "assert".to_owned(),
        "".to_owned(), false, None, None, ast::Visibility::Private(ast::Loc(0, 0)),
        vec!( Parameter {
            name: "arg0".to_owned(),
            ty: resolver::Type::Primitive(ast::PrimitiveType::Bool)
        } ), vec!(), &ns);

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());

    let cond = vartab.add(&ast::Identifier{
        loc: ast::Loc(0, 0),
        name: "arg0".to_owned()
    }, argty, &mut errors).unwrap();

    cfg.add(&mut vartab, Instr::FuncArg{ res: cond, arg: 0 });
    cfg.add(&mut vartab, Instr::BranchCond{
        cond: Expression::Variable(ast::Loc(0, 0), cond),
        true_, false_
    });

    cfg.set_basic_block(true_);
    cfg.add(&mut vartab, Instr::Return{ value: Vec::new() });

    cfg.set_basic_block(false_);
    cfg.add(&mut vartab, Instr::AssertFailure{  });

    cfg.vars = vartab.drain();

    assert.cfg = Some(Box::new(cfg));

    let pos = ns.functions.len();

    ns.functions.push(assert);

    ns.add_symbol(&id, resolver::Symbol::Function(vec![(id.loc, pos)]), &mut errors);

}
