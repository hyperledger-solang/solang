// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ASTFunction;
use crate::ssa_ir::cfg::Block;
use crate::ssa_ir::cfg::Cfg;
use crate::ssa_ir::printer::Printer;
use std::io::Write;

impl Printer {
    pub fn print_cfg(&self, f: &mut dyn Write, cfg: &Cfg) -> std::io::Result<()> {
        let function_no = match cfg.function_no {
            ASTFunction::SolidityFunction(no) => format!("sol#{}", no),
            ASTFunction::YulFunction(no) => format!("yul#{}", no),
            ASTFunction::None => "none".to_string(),
        };

        let access_ctl = if cfg.public { "public" } else { "private" };

        write!(f, "{} {} {} {} ", access_ctl, cfg.ty, function_no, cfg.name)?;

        write!(f, "(")?;
        for (i, param) in cfg.params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param.ty)?;
        }
        write!(f, ")")?;

        if !cfg.returns.is_empty() {
            write!(f, " returns (")?;
            for (i, ret) in cfg.returns.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", ret.ty)?;
            }
            write!(f, ")")?;
        }
        writeln!(f, ":")?;

        for (i, block) in cfg.blocks.iter().enumerate() {
            writeln!(f, "block#{} {}:", i, block.name)?;
            self.print_block(f, block)?;
            writeln!(f)?;
        }
        Ok(())
    }

    pub fn print_block(&self, f: &mut dyn Write, block: &Block) -> std::io::Result<()> {
        for insn in &block.instructions {
            write!(f, "    ")?;
            self.print_insn(f, insn)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
