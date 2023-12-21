// SPDX-License-Identifier: Apache-2.0

use crate::lir_tests::helpers::{identifier, num_literal};
use crate::{num_literal, stringfy_lir};
use indexmap::IndexMap;
use solang::lir::lir_type::{LIRType, Type};
use solang::lir::printer::Printer;
use solang::lir::vartable::Var;
use solang::lir::{instructions::Instruction, vartable::Vartable, Block, LIR};
use solang::sema::ast::{self, Parameter};
use solang_parser::pt::{Identifier, Loc};

#[test]
fn test_stringfy_cfg() {
    let cfg = new_cfg(vec![
        new_block(
            String::from("entry"),
            vec![
                Instruction::LoadStorage {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    res: 0,
                    storage: identifier(3),
                },
                Instruction::BranchCond {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    cond: identifier(0),
                    true_block: 1,
                    false_block: 2,
                },
            ],
        ),
        new_block(
            String::from("blk1"),
            vec![
                Instruction::Print {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    operand: num_literal!(1),
                },
                Instruction::Branch { loc: /*missing from cfg*/ Loc::Codegen, block: 3 },
            ],
        ),
        new_block(
            String::from("blk2"),
            vec![
                Instruction::Print {
                    loc: /*missing from cfg*/ Loc::Codegen,
                    operand: num_literal!(2),
                },
                Instruction::Branch { loc: /*missing from cfg*/ Loc::Codegen, block: 3 },
            ],
        ),
        new_block(
            String::from("exit"),
            vec![Instruction::ReturnData {
                loc: /*missing from cfg*/ Loc::Codegen,
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
            ty: LIRType {
                lir_type: Type::Int(32),
                ast_type: ast::Type::Int(32),
            },
            name: String::from("x"),
        },
    );
    var_table.vars.insert(
        3,
        Var {
            id: 1,
            ty: LIRType {
                lir_type: Type::StoragePtr(false, Box::new(Type::Int(32))),
                ast_type: ast::Type::Int(32),
            },
            name: String::from("st"),
        },
    );
    let printer = Printer::new(&var_table);

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
        stringfy_lir!(&printer, &cfg).trim()
    )
}

fn new_block(name: String, instructions: Vec<Instruction>) -> Block {
    Block { name, instructions }
}

fn new_cfg(blocks: Vec<Block>) -> LIR {
    LIR {
        name: String::from("test_cfg"),
        function_no: solang::codegen::cfg::ASTFunction::SolidityFunction(0),
        ty: solang_parser::pt::FunctionTy::Function,
        public: true,
        nonpayable: false,
        vartable: new_vartable(),
        params: vec![
            new_parameter(
                String::from("a"),
                LIRType {
                    ast_type: ast::Type::Int(32),
                    lir_type: Type::Int(32),
                },
            ),
            new_parameter(
                String::from("b"),
                LIRType {
                    ast_type: ast::Type::Int(32),
                    lir_type: Type::Int(32),
                },
            ),
        ],
        returns: vec![new_parameter(
            String::from("c"),
            LIRType {
                ast_type: ast::Type::Int(32),
                lir_type: Type::Int(32),
            },
        )],
        blocks,
        selector: vec![],
    }
}

fn new_parameter(name: String, ty: LIRType) -> Parameter<LIRType> {
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
