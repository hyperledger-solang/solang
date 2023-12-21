// SPDX-License-Identifier: Apache-2.0

use super::expressions::Operand;
use super::lir_type::LIRType;
use crate::codegen::cfg::ASTFunction;
use crate::lir::vartable::Vartable;
use crate::lir::{Block, LIR};
use std::io::Write;

pub mod expression;
pub mod instruction;

pub struct Printer<'a> {
    vartable: &'a Vartable,
}

impl<'a> Printer<'a> {
    /// Create a new Printer with a reference to the Vartable.
    pub fn new(vartable: &'a Vartable) -> Self {
        Self { vartable }
    }

    /// get a variable name by its unique identifier.
    pub(crate) fn get_var_name(&self, id: &usize) -> &str {
        self.vartable.get_name(id)
    }

    /// get a variable type by its unique identifier.
    pub(crate) fn get_var_type(&self, id: &usize) -> &LIRType {
        self.vartable.get_type(id)
    }

    /// get a variable operand by its unique identifier.
    pub(crate) fn get_var_operand(&self, id: &usize) -> Operand {
        self.vartable.get_operand(
            id,
            // the location is not important for printing
            solang_parser::pt::Loc::Codegen,
        )
    }

    pub fn print_lir(&self, f: &mut dyn Write, cfg: &LIR) {
        let function_no = match cfg.function_no {
            ASTFunction::SolidityFunction(no) => format!("sol#{}", no),
            ASTFunction::YulFunction(no) => format!("yul#{}", no),
            ASTFunction::None => "none".to_string(),
        };

        let access_ctl = if cfg.public { "public" } else { "private" };

        write!(f, "{} {} {} {} ", access_ctl, cfg.ty, function_no, cfg.name).unwrap();

        write!(f, "(").unwrap();
        for (i, param) in cfg.params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ").unwrap();
            }
            write!(f, "{}", param.ty).unwrap();
        }
        write!(f, ")").unwrap();

        if !cfg.returns.is_empty() {
            write!(f, " returns (").unwrap();
            for (i, ret) in cfg.returns.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ").unwrap();
                }
                write!(f, "{}", ret.ty).unwrap();
            }
            write!(f, ")").unwrap();
        }
        writeln!(f, ":").unwrap();

        for (i, block) in cfg.blocks.iter().enumerate() {
            writeln!(f, "block#{} {}:", i, block.name).unwrap();
            self.print_block(f, block);
            writeln!(f).unwrap();
        }
    }

    pub fn print_block(&self, f: &mut dyn Write, block: &Block) {
        for insn in &block.instructions {
            write!(f, "    ").unwrap();
            self.print_instruction(f, insn);
            writeln!(f).unwrap();
        }
    }
}
