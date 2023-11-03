// SPDX-License-Identifier: Apache-2.0

use crate::ssa_ir_tests::helpers::{identifier, num_literal};
use crate::{num_literal, stringfy_cfg};
use indexmap::IndexMap;
use num_bigint::BigInt;
use solang::sema::ast::Parameter;
use solang::ssa_ir::printer::Printer;
use solang::ssa_ir::vartable::{Storage, Var};
use solang::ssa_ir::{
    cfg::{Block, Cfg},
    instructions::Insn,
    ssa_type::Type,
    vartable::Vartable,
};
use solang_parser::pt::{Identifier, Loc};

#[test]
fn test_stringfy_cfg() {
    let cfg = new_cfg(vec![
        new_block(
            String::from("entry"),
            vec![
                Insn::LoadStorage {
                    res: 0,
                    storage: identifier(3),
                },
                Insn::BranchCond {
                    cond: identifier(0),
                    true_block: 1,
                    false_block: 2,
                },
            ],
        ),
        new_block(
            String::from("blk1"),
            vec![
                Insn::Print {
                    operand: num_literal!(1),
                },
                Insn::Branch { block: 3 },
            ],
        ),
        new_block(
            String::from("blk2"),
            vec![
                Insn::Print {
                    operand: num_literal!(2),
                },
                Insn::Branch { block: 3 },
            ],
        ),
        new_block(
            String::from("exit"),
            vec![Insn::ReturnData {
                data: identifier(0),
                data_len: num_literal!(1),
            }],
        ),
    ]);

    let mut var_table = Vartable {
        vars: IndexMap::new(),
        args: IndexMap::new(),
        next_id: 0,
    };

    // construct a index map for the vartable
    var_table.vars.insert(
        0,
        Var {
            id: 0,
            ty: Type::Int(32),
            name: String::from("x"),
            storage: Storage::Local,
        },
    );
    var_table.vars.insert(
        3,
        Var {
            id: 1,
            ty: Type::StoragePtr(false, Box::new(Type::Int(32))),
            name: String::from("st"),
            storage: Storage::Contract(BigInt::from(0)),
        },
    );
    let printer = Printer {
        vartable: Box::new(var_table),
    };

    assert_eq!(
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            "public function sol#0 test_cfg (int32, int32) returns (int32):",
            "block#0 entry:",
            "    int32 %x = load_storage storage_ptr<int32>(%st);",
            "    cbr int32(%x) block#1 else block#2;",
            "",
            "block#1 blk1:",
            "    print uint8(1);",
            "    br block#3;",
            "",
            "block#2 blk2:",
            "    print uint8(2);",
            "    br block#3;",
            "",
            "block#3 exit:",
            "    return_data int32(%x) of length uint8(1);"
        ),
        stringfy_cfg!(&printer, &cfg).trim()
    )
}

fn new_block(name: String, instructions: Vec<Insn>) -> Block {
    Block { name, instructions }
}

fn new_cfg(blocks: Vec<Block>) -> Cfg {
    Cfg {
        name: String::from("test_cfg"),
        function_no: solang::codegen::cfg::ASTFunction::SolidityFunction(0),
        ty: solang_parser::pt::FunctionTy::Function,
        public: true,
        nonpayable: false,
        vartable: new_vartable(),
        params: vec![
            new_parameter(String::from("a"), Type::Int(32)),
            new_parameter(String::from("b"), Type::Int(32)),
        ],
        returns: vec![new_parameter(String::from("c"), Type::Int(32))],
        blocks,
        selector: vec![],
    }
}

fn new_parameter(name: String, ty: Type) -> Parameter<Type> {
    Parameter {
        loc: Loc::Codegen,
        id: Some(Identifier::new(name)),
        ty,
        annotation: None,
        indexed: false,
        infinite_size: false,
        readonly: false,
        recursive: false,
        ty_loc: None,
    }
}

fn new_vartable() -> Vartable {
    Vartable {
        vars: IndexMap::new(),
        args: IndexMap::new(),
        next_id: 0,
    }
}
