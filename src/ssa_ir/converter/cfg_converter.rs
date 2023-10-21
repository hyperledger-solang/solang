// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::ssa_ir::cfg::{Block, Cfg};
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::ssa_type::Parameter;

impl Converter<'_> {
    pub fn get_ssa_ir_cfg(&self) -> Result<Cfg, String> {
        let mut vartable = self.from_vars(&self.cfg.vars)?;

        let blocks = self
            .cfg
            .blocks
            .iter()
            .map(|block| self.from_basic_block(block, &mut vartable))
            .collect::<Result<Vec<Block>, String>>()?;

        let params = self
            .cfg
            .params
            .iter()
            .map(|p| self.from_ast_parameter(p))
            .collect::<Result<Vec<Parameter>, String>>()?;

        let returns = self
            .cfg
            .returns
            .iter()
            .map(|p| self.from_ast_parameter(p))
            .collect::<Result<Vec<Parameter>, String>>()?;

        let ssa_ir_cfg = Cfg {
            name: self.cfg.name.clone(),
            function_no: self.cfg.function_no,
            params: Arc::new(params),
            returns: Arc::new(returns),
            vartable,
            blocks,
            nonpayable: self.cfg.nonpayable,
            public: self.cfg.public,
            ty: self.cfg.ty,
            selector: self.cfg.selector.clone(),
        };

        Ok(ssa_ir_cfg)
    }
}
