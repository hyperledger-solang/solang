// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::BasicBlock;
use crate::sema::ast::{self, Parameter};
use crate::ssa_ir::cfg::{Block, Cfg};
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::ssa_type;
use crate::ssa_ir::vartable::Vartable;

impl Converter<'_> {
    pub fn get_three_address_code_cfg(&self) -> Result<Cfg, String> {
        let mut vartable = self.from_vars(&self.cfg.vars)?;

        let blocks = self
            .cfg
            .blocks
            .iter()
            .map(|block| self.lowering_basic_block(block, &mut vartable))
            .collect::<Result<Vec<Block>, String>>()?;

        let params = self
            .cfg
            .params
            .iter()
            .map(|p| self.to_ssa_typed_parameter(p))
            .collect::<Result<Vec<Parameter<ssa_type::Type>>, String>>()?;

        let returns = self
            .cfg
            .returns
            .iter()
            .map(|p| self.to_ssa_typed_parameter(p))
            .collect::<Result<Vec<Parameter<ssa_type::Type>>, String>>()?;

        let ssa_ir_cfg = Cfg {
            name: self.cfg.name.clone(),
            function_no: self.cfg.function_no,
            params,
            returns,
            vartable,
            blocks,
            nonpayable: self.cfg.nonpayable,
            public: self.cfg.public,
            ty: self.cfg.ty,
            selector: self.cfg.selector.clone(),
        };

        Ok(ssa_ir_cfg)
    }

    fn lowering_basic_block(
        &self,
        basic_block: &BasicBlock,
        vartable: &mut Vartable,
    ) -> Result<Block, String> {
        let mut instructions = vec![];
        for insn in &basic_block.instr {
            let insns = self.lowering_instr(insn, vartable)?;
            insns.into_iter().for_each(|i| instructions.push(i));
        }

        let block = Block {
            name: basic_block.name.clone(),
            instructions,
        };

        Ok(block)
    }

    fn to_ssa_typed_parameter(
        &self,
        param: &Parameter<ast::Type>,
    ) -> Result<Parameter<ssa_type::Type>, String> {
        let ty = self.from_ast_type(&param.ty)?;
        Ok(Parameter {
            loc: param.loc,
            id: param.id.clone(),
            ty,
            ty_loc: param.ty_loc,
            indexed: param.indexed,
            readonly: param.readonly,
            infinite_size: param.infinite_size,
            recursive: param.recursive,
            annotation: param.annotation.clone(),
        })
    }
}
