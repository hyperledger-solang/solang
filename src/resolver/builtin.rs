use super::cfg::{ControlFlowGraph, Instr, Vartable};
use super::expression::Expression;
use super::{FunctionDecl, Namespace, Parameter};
use parser::ast;
use resolver;

pub fn add_builtin_function(ns: &mut Namespace, contract_no: usize) {
    add_assert(ns, contract_no);
    add_print(ns, contract_no);
    add_revert(ns, contract_no);
    add_require(ns, contract_no);
}

fn add_assert(ns: &mut Namespace, contract_no: usize) {
    let argty = resolver::Type::Bool;
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "assert".to_owned(),
    };

    let mut assert = FunctionDecl::new(
        ast::Loc(0, 0),
        "assert".to_owned(),
        vec![],
        false,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![Parameter {
            name: "arg0".to_owned(),
            ty: resolver::Type::Bool,
        }],
        vec![],
        ns,
    );

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());

    let cond = vartab
        .add(
            &ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "arg0".to_owned(),
            },
            argty,
            &mut errors,
        )
        .unwrap();

    cfg.add(&mut vartab, Instr::FuncArg { res: cond, arg: 0 });
    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::Variable(ast::Loc(0, 0), cond),
            true_,
            false_,
        },
    );

    cfg.set_basic_block(true_);
    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    cfg.set_basic_block(false_);
    cfg.add(&mut vartab, Instr::AssertFailure { expr: None });

    cfg.vars = vartab.drain();

    assert.cfg = Some(Box::new(cfg));

    let pos = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(assert);

    ns.add_symbol(
        Some(contract_no),
        &id,
        resolver::Symbol::Function(vec![(id.loc, pos)]),
        &mut errors,
    );
}

fn add_print(ns: &mut Namespace, contract_no: usize) {
    let argty = resolver::Type::String;
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "print".to_owned(),
    };

    let mut assert = FunctionDecl::new(
        ast::Loc(0, 0),
        "print".to_owned(),
        vec![],
        false,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![Parameter {
            name: "arg0".to_owned(),
            ty: resolver::Type::String,
        }],
        vec![],
        ns,
    );

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let cond = vartab
        .add(
            &ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "arg0".to_owned(),
            },
            argty,
            &mut errors,
        )
        .unwrap();

    cfg.add(&mut vartab, Instr::FuncArg { res: cond, arg: 0 });
    cfg.add(
        &mut vartab,
        Instr::Print {
            expr: Expression::Variable(ast::Loc(0, 0), cond),
        },
    );
    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });
    cfg.vars = vartab.drain();

    assert.cfg = Some(Box::new(cfg));

    let pos = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(assert);

    ns.add_symbol(
        Some(contract_no),
        &id,
        resolver::Symbol::Function(vec![(id.loc, pos)]),
        &mut errors,
    );
}

fn add_require(ns: &mut Namespace, contract_no: usize) {
    let argty = resolver::Type::Bool;
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "require".to_owned(),
    };

    let mut require = FunctionDecl::new(
        ast::Loc(0, 0),
        "require".to_owned(),
        vec![],
        false,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![
            Parameter {
                name: "condition".to_owned(),
                ty: resolver::Type::Bool,
            },
            Parameter {
                name: "ReasonCode".to_owned(),
                ty: resolver::Type::String,
            },
        ],
        vec![],
        ns,
    );

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());

    let error = vartab
        .add(
            &ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "ReasonCode".to_owned(),
            },
            resolver::Type::String,
            &mut errors,
        )
        .unwrap();

    cfg.add(&mut vartab, Instr::FuncArg { res: error, arg: 1 });

    let cond = vartab
        .add(
            &ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "condition".to_owned(),
            },
            argty,
            &mut errors,
        )
        .unwrap();

    cfg.add(&mut vartab, Instr::FuncArg { res: cond, arg: 0 });
    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::Variable(ast::Loc(0, 0), cond),
            true_,
            false_,
        },
    );

    cfg.set_basic_block(true_);
    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    cfg.set_basic_block(false_);
    cfg.add(
        &mut vartab,
        Instr::AssertFailure {
            expr: Some(Expression::Variable(ast::Loc(0, 0), error)),
        },
    );

    cfg.vars = vartab.drain();

    require.cfg = Some(Box::new(cfg));

    let pos_with_reason = ns.contracts[contract_no].functions.len();

    let argty = resolver::Type::Bool;

    ns.contracts[contract_no].functions.push(require);

    let mut require = FunctionDecl::new(
        ast::Loc(0, 0),
        "require".to_owned(),
        vec![],
        false,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![Parameter {
            name: "condition".to_owned(),
            ty: resolver::Type::Bool,
        }],
        vec![],
        ns,
    );

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());

    let cond = vartab
        .add(
            &ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "condition".to_owned(),
            },
            argty,
            &mut errors,
        )
        .unwrap();

    cfg.add(&mut vartab, Instr::FuncArg { res: cond, arg: 0 });
    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::Variable(ast::Loc(0, 0), cond),
            true_,
            false_,
        },
    );

    cfg.set_basic_block(true_);
    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    cfg.set_basic_block(false_);
    cfg.add(&mut vartab, Instr::AssertFailure { expr: None });

    cfg.vars = vartab.drain();

    require.cfg = Some(Box::new(cfg));

    let pos_without_reason = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(require);

    ns.add_symbol(
        Some(contract_no),
        &id,
        resolver::Symbol::Function(vec![
            (id.loc, pos_with_reason),
            (id.loc, pos_without_reason),
        ]),
        &mut errors,
    );
}

fn add_revert(ns: &mut Namespace, contract_no: usize) {
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "revert".to_owned(),
    };

    let mut revert = FunctionDecl::new(
        ast::Loc(0, 0),
        "revert".to_owned(),
        vec![],
        false,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![Parameter {
            name: "ReasonCode".to_owned(),
            ty: resolver::Type::String,
        }],
        vec![],
        ns,
    );

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let error = vartab
        .add(
            &ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "ReasonCode".to_owned(),
            },
            resolver::Type::String,
            &mut errors,
        )
        .unwrap();

    cfg.add(&mut vartab, Instr::FuncArg { res: error, arg: 0 });

    cfg.add(
        &mut vartab,
        Instr::AssertFailure {
            expr: Some(Expression::Variable(ast::Loc(0, 0), error)),
        },
    );

    cfg.vars = vartab.drain();

    revert.cfg = Some(Box::new(cfg));

    let pos_with_arg = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(revert);

    // now add variant with no argument
    let mut revert = FunctionDecl::new(
        ast::Loc(0, 0),
        "revert".to_owned(),
        vec![],
        false,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![],
        vec![],
        ns,
    );

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    cfg.add(&mut vartab, Instr::AssertFailure { expr: None });

    cfg.vars = vartab.drain();

    revert.cfg = Some(Box::new(cfg));

    let pos_with_no_arg = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(revert);

    ns.add_symbol(
        Some(contract_no),
        &id,
        resolver::Symbol::Function(vec![(id.loc, pos_with_arg), (id.loc, pos_with_no_arg)]),
        &mut errors,
    );
}
