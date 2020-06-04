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
    add_selfdestruct(ns, contract_no);
}

fn add_assert(ns: &mut Namespace, contract_no: usize) {
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "assert".to_owned(),
    };

    let mut assert = FunctionDecl::new(
        ast::Loc(0, 0),
        "assert".to_owned(),
        vec![],
        ast::FunctionTy::Function,
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

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::FunctionArg(ast::Loc(0, 0), 0),
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
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "print".to_owned(),
    };

    let mut assert = FunctionDecl::new(
        ast::Loc(0, 0),
        "print".to_owned(),
        vec![],
        ast::FunctionTy::Function,
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

    cfg.add(
        &mut vartab,
        Instr::Print {
            expr: Expression::FunctionArg(ast::Loc(0, 0), 0),
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
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "require".to_owned(),
    };

    let mut require = FunctionDecl::new(
        ast::Loc(0, 0),
        "require".to_owned(),
        vec![],
        ast::FunctionTy::Function,
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

    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::FunctionArg(ast::Loc(0, 0), 0),
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
            expr: Some(Expression::FunctionArg(ast::Loc(0, 0), 1)),
        },
    );

    cfg.vars = vartab.drain();

    require.cfg = Some(Box::new(cfg));

    let pos_with_reason = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(require);

    let mut require = FunctionDecl::new(
        ast::Loc(0, 0),
        "require".to_owned(),
        vec![],
        ast::FunctionTy::Function,
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

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: Expression::FunctionArg(ast::Loc(0, 0), 0),
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
        ast::FunctionTy::Function,
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

    revert.noreturn = true;

    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    cfg.add(
        &mut vartab,
        Instr::AssertFailure {
            expr: Some(Expression::FunctionArg(ast::Loc(0, 0), 0)),
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
        ast::FunctionTy::Function,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![],
        vec![],
        ns,
    );

    revert.noreturn = true;

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

fn add_selfdestruct(ns: &mut Namespace, contract_no: usize) {
    let id = ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "selfdestruct".to_owned(),
    };

    let mut selfdestruct = FunctionDecl::new(
        ast::Loc(0, 0),
        "selfdestruct".to_owned(),
        vec![],
        ast::FunctionTy::Function,
        None,
        None,
        ast::Visibility::Private(ast::Loc(0, 0)),
        vec![Parameter {
            name: "recipient".to_owned(),
            ty: resolver::Type::Address(true),
        }],
        vec![],
        ns,
    );

    selfdestruct.noreturn = true;

    let mut errors = Vec::new();
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    cfg.add(
        &mut vartab,
        Instr::SelfDestruct {
            recipient: Expression::FunctionArg(ast::Loc(0, 0), 0),
        },
    );
    cfg.add(&mut vartab, Instr::Unreachable);
    cfg.vars = vartab.drain();

    selfdestruct.cfg = Some(Box::new(cfg));

    let pos = ns.contracts[contract_no].functions.len();

    ns.contracts[contract_no].functions.push(selfdestruct);

    ns.add_symbol(
        Some(contract_no),
        &id,
        resolver::Symbol::Function(vec![(id.loc, pos)]),
        &mut errors,
    );
}
