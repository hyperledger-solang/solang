// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::vartable::Vartable;
use crate::codegen::yul::expression::expression;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::ast::ArrayLength;
use crate::sema::ast::{Contract, Layout, Mutability, Namespace, Type, Variable};
use crate::sema::yul::ast;
use crate::sema::yul::ast::YulSuffix;
use crate::{sema, Target};
use num_bigint::{BigInt, Sign};
use once_cell::unsync::OnceCell;
use solang_parser::pt::{ContractTy, Loc, StorageLocation, Visibility};

#[test]
fn bool_literal() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::BoolLiteral(loc, true, Type::Bool);
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(res, Expression::BoolLiteral(loc, true));

    let expr = ast::YulExpression::BoolLiteral(loc, true, Type::Uint(32));
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(loc, Type::Uint(32), BigInt::from(1))
    );

    let expr = ast::YulExpression::BoolLiteral(loc, false, Type::Uint(32));
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(loc, Type::Uint(32), BigInt::from(0))
    );
}

#[test]
fn number_literal() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::NumberLiteral(loc, BigInt::from(32), Type::Uint(256));
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(loc, Type::Uint(256), BigInt::from(32))
    );
}

#[test]
fn string_literal() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::StringLiteral(loc, vec![0, 3, 255, 127], Type::Uint(128));
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(
            loc,
            Type::Uint(128),
            BigInt::from_bytes_be(Sign::Plus, &[0, 3, 255, 127])
        )
    );
}

#[test]
fn yul_local_variable() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::YulLocalVariable(loc, Type::Int(16), 5);
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(res, Expression::Variable(loc, Type::Int(16), 5));
}

#[test]
fn contract_constant_variable() {
    let loc = Loc::File(1, 2, 3);
    let mut ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let var = Variable {
        tags: vec![],
        name: "const_var".to_string(),
        loc,
        ty: Type::Uint(64),
        visibility: Visibility::Public(None),
        constant: false,
        immutable: false,
        initializer: Some(sema::ast::Expression::NumberLiteral {
            loc,
            ty: Type::Uint(64),
            value: BigInt::from(64),
        }),
        assigned: false,
        read: false,
    };

    let contract = Contract {
        tags: vec![],
        loc,
        ty: ContractTy::Contract(loc),
        name: "".to_string(),
        bases: vec![],
        using: vec![],
        layout: vec![],
        fixed_layout_size: Default::default(),
        functions: vec![],
        all_functions: Default::default(),
        virtual_functions: Default::default(),
        yul_functions: vec![],
        variables: vec![var],
        creates: vec![],
        emits_events: vec![],
        initializer: None,
        default_constructor: None,
        cfg: vec![],
        code: OnceCell::new(),
        instantiable: true,
        dispatch_no: 0,
        program_id: None,
    };
    ns.contracts.push(contract);

    let expr = ast::YulExpression::ConstantVariable(loc, Type::Uint(64), Some(0), 0);
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(64))
    );
}

#[test]
fn global_constant_variable() {
    let loc = Loc::File(1, 2, 3);
    let mut ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let var = Variable {
        tags: vec![],
        name: "const_var".to_string(),
        loc,
        ty: Type::Uint(64),
        visibility: Visibility::Public(None),
        constant: false,
        immutable: false,
        initializer: Some(sema::ast::Expression::NumberLiteral {
            loc,
            ty: Type::Uint(64),
            value: BigInt::from(64),
        }),
        assigned: false,
        read: false,
    };
    ns.constants.push(var);
    let expr = ast::YulExpression::ConstantVariable(loc, Type::Uint(64), None, 0);
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(loc, Type::Uint(64), BigInt::from(64))
    );
}

#[test]
#[should_panic]
fn storage_variable() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::StorageVariable(loc, Type::Bool, 0, 0);
    let _ = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}

#[test]
#[should_panic]
fn storage_variable_reference() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SolidityLocalVariable(
        loc,
        Type::Int(32),
        Some(StorageLocation::Storage(loc)),
        0,
    );
    let _ = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}

#[test]
fn solidity_local_variable() {
    let loc = Loc::File(1, 2, 3);
    let ns = Namespace::new(Target::Solana);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SolidityLocalVariable(loc, Type::Uint(32), None, 7);
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(res, Expression::Variable(loc, Type::Uint(32), 7));
}

#[test]
fn slot_suffix() {
    let mut ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let layout = Layout {
        slot: BigInt::from(2),
        contract_no: 0,
        var_no: 0,
        ty: Type::Uint(256),
    };
    let contract = Contract {
        tags: vec![],
        loc: Loc::Builtin,
        ty: ContractTy::Contract(loc),
        name: "".to_string(),
        bases: vec![],
        using: vec![],
        layout: vec![layout],
        fixed_layout_size: Default::default(),
        functions: vec![],
        all_functions: Default::default(),
        virtual_functions: Default::default(),
        yul_functions: vec![],
        variables: vec![],
        creates: vec![],
        emits_events: vec![],
        initializer: None,
        default_constructor: None,
        cfg: vec![],
        code: OnceCell::new(),
        instantiable: true,
        dispatch_no: 0,
        program_id: None,
    };
    ns.contracts.push(contract);

    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::StorageVariable(
            loc,
            Type::Uint(256),
            0,
            0,
        )),
        YulSuffix::Slot,
    );
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(Loc::File(1, 2, 3), Type::Uint(256), BigInt::from(2))
    );

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Uint(16),
            Some(StorageLocation::Storage(loc)),
            0,
        )),
        YulSuffix::Slot,
    );
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(res, Expression::Variable(loc, Type::Uint(256), 0));
}

#[test]
#[should_panic]
fn slot_suffix_panic() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Int(32),
            None,
            2,
        )),
        YulSuffix::Slot,
    );

    let _res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}

#[test]
fn offset_suffix() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::StorageVariable(
            loc,
            Type::Int(32),
            1,
            0,
        )),
        YulSuffix::Offset,
    );

    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(256), BigInt::from(0))
    );

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Uint(24),
            Some(StorageLocation::Storage(loc)),
            0,
        )),
        YulSuffix::Offset,
    );
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(256), BigInt::from(0))
    );

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Array(Box::new(Type::Uint(256)), vec![ArrayLength::Dynamic]),
            Some(StorageLocation::Calldata(loc)),
            1,
        )),
        YulSuffix::Offset,
    );
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::Cast(
            loc,
            Type::Uint(256),
            Box::new(Expression::Variable(
                loc,
                Type::Array(Box::new(Type::Uint(256)), vec![ArrayLength::Dynamic]),
                1
            ))
        )
    );
}

#[test]
#[should_panic]
fn offset_suffix_panic_calldata() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Array(
                Box::new(Type::Uint(32)),
                vec![ArrayLength::Dynamic, ArrayLength::Fixed(BigInt::from(3))],
            ),
            Some(StorageLocation::Calldata(loc)),
            3,
        )),
        YulSuffix::Offset,
    );

    let _res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}

#[test]
#[should_panic]
fn offset_suffix_panic_other() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::YulLocalVariable(loc, Type::Int(32), 3)),
        YulSuffix::Offset,
    );

    let _res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}

#[test]
fn length_suffix() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Array(
                Box::new(Type::Uint(32)),
                vec![
                    ArrayLength::Dynamic,
                    ArrayLength::Fixed(BigInt::from(3)),
                    ArrayLength::Dynamic,
                ],
            ),
            Some(StorageLocation::Calldata(loc)),
            3,
        )),
        YulSuffix::Length,
    );

    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::Builtin(
            loc,
            vec![Type::Uint(32)],
            Builtin::ArrayLength,
            vec![Expression::Variable(
                loc,
                Type::Array(
                    Box::new(Type::Uint(32)),
                    vec![
                        ArrayLength::Dynamic,
                        ArrayLength::Fixed(BigInt::from(3)),
                        ArrayLength::Dynamic
                    ]
                ),
                3
            )]
        )
    );
}

#[test]
#[should_panic]
fn length_suffix_panic() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Array(
                Box::new(Type::Uint(32)),
                vec![ArrayLength::Dynamic, ArrayLength::Fixed(BigInt::from(3))],
            ),
            Some(StorageLocation::Calldata(loc)),
            3,
        )),
        YulSuffix::Length,
    );

    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
    assert_eq!(
        res,
        Expression::Builtin(
            loc,
            vec![Type::Uint(256)],
            Builtin::ArrayLength,
            vec![Expression::Variable(
                loc,
                Type::Array(
                    Box::new(Type::Uint(32)),
                    vec![
                        ArrayLength::Dynamic,
                        ArrayLength::Fixed(BigInt::from(3)),
                        ArrayLength::Dynamic
                    ]
                ),
                3
            )]
        )
    );
}

#[test]
fn selector_suffix() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::ExternalFunction {
                mutability: Mutability::Pure(loc),
                params: vec![],
                returns: vec![],
            },
            None,
            4,
        )),
        YulSuffix::Selector,
    );
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);

    assert_eq!(
        res,
        Expression::Load(
            loc,
            Type::FunctionSelector,
            Box::new(Expression::StructMember(
                loc,
                Type::Ref(Box::new(Type::FunctionSelector)),
                Box::new(Expression::Variable(
                    loc,
                    Type::ExternalFunction {
                        mutability: Mutability::Pure(loc),
                        params: vec![],
                        returns: vec![],
                    },
                    4
                )),
                0
            ))
        )
    );
}

#[test]
#[should_panic]
fn selector_suffix_panic() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Bool,
            None,
            4,
        )),
        YulSuffix::Selector,
    );
    let _res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}

#[test]
fn address_suffix() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::ExternalFunction {
                mutability: Mutability::Pure(loc),
                params: vec![],
                returns: vec![],
            },
            None,
            4,
        )),
        YulSuffix::Address,
    );
    let res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);

    assert_eq!(
        res,
        Expression::Load(
            loc,
            Type::Address(false),
            Box::new(Expression::StructMember(
                loc,
                Type::Ref(Box::new(Type::Address(false))),
                Box::new(Expression::Variable(
                    loc,
                    Type::ExternalFunction {
                        mutability: Mutability::Pure(loc),
                        params: vec![],
                        returns: vec![]
                    },
                    4
                )),
                1
            ))
        )
    );
}

#[test]
#[should_panic]
fn address_suffix_panic() {
    let ns = Namespace::new(Target::Solana);
    let loc = Loc::File(1, 2, 3);
    let mut vartab = Vartable::new(2);
    let mut cfg = ControlFlowGraph::placeholder();
    let opt = Options::default();

    let expr = ast::YulExpression::SuffixAccess(
        loc,
        Box::new(ast::YulExpression::SolidityLocalVariable(
            loc,
            Type::Bool,
            None,
            4,
        )),
        YulSuffix::Address,
    );
    let _res = expression(&expr, 0, &ns, &mut vartab, &mut cfg, &opt);
}
