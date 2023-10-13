use std::sync::Arc;

use crate::num_literal;
use crate::ssa_ir_tests::helpers::{identifier, num_literal};
use indexmap::IndexMap;
use solang::ssa_ir::{
    cfg::{Block, Cfg},
    insn::Insn,
    ssa_type::{Parameter, Type},
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

    assert_eq!(
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            "public function sol#0 test_cfg (int32, int32) returns (int32):",
            "block entry:",
            "    %0 = load_storage %3;",
            "    cbr %0 block#1 else block#2;",
            "block blk1:",
            "    print uint8(1);",
            "    br block#3;",
            "block blk2:",
            "    print uint8(2);",
            "    br block#3;",
            "block exit:",
            "    return %0 of length uint8(1);"
        ),
        format!("{}", cfg)
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
        params: Arc::new(vec![
            new_parameter(String::from("a"), Type::Int(32)),
            new_parameter(String::from("b"), Type::Int(32)),
        ]),
        returns: Arc::new(vec![new_parameter(String::from("c"), Type::Int(32))]),
        blocks,
        selector: vec![],
    }
}

fn new_parameter(name: String, ty: Type) -> Parameter {
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
        next_id: 0,
    }
}
