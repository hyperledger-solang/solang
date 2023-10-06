use crate::codegen::cfg::BasicBlock;
use crate::ssa_ir::cfg::Block;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::vartable::Vartable;

impl Converter {
    pub(crate) fn from_basic_block(basic_block: &BasicBlock, vartable: &mut Vartable) -> Result<Block, &'static str> {
        let mut instructions = Vec::new();
        for insn in &basic_block.instr {
            for insn in Converter::from_instr(insn, vartable)? {
                instructions.push(insn);
            }
        }

        let block = Block {
            name: basic_block.name.clone(),
            instructions
        };

        Ok(block)
    }
}