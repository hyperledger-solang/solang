// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::BasicBlock;
use crate::ssa_ir::cfg::Block;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub fn from_basic_block(
        &self,
        basic_block: &BasicBlock,
        vartable: &mut Vartable,
    ) -> Result<Block, String> {
        let mut instructions = vec![];
        for insn in &basic_block.instr {
            let insns = self.from_instr(insn, vartable)?;
            insns.into_iter().for_each(|i| instructions.push(i));
        }

        let block = Block {
            name: basic_block.name.clone(),
            instructions,
        };

        Ok(block)
    }
}
