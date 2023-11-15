// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ASTFunction;
use crate::lir::printer::Printer;
use crate::lir::{Block, LIR};
use std::io::Write;

impl Printer {
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
            self.print_insn(f, insn);
            writeln!(f).unwrap();
        }
    }
}
