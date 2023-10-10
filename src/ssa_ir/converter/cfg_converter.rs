use std::sync::Arc;

use crate::codegen::cfg::ControlFlowGraph;
use crate::ssa_ir::cfg::{Block, Cfg};
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::ssa_type::Parameter;
use crate::ssa_ir::vartable::Vartable;

impl Converter {
    pub fn from_control_flow_graph(cfg: &ControlFlowGraph) -> Result<Cfg, &'static str> {
        let mut vartable = Vartable::try_from(&cfg.vars)?;

        let blocks = cfg
            .blocks
            .iter()
            .map(|block| Converter::from_basic_block(block, &mut vartable))
            .collect::<Result<Vec<Block>, &'static str>>()?;

        let params = cfg
            .params
            .iter()
            .map(|p| Parameter::try_from(p))
            .collect::<Result<Vec<Parameter>, &'static str>>()?;

        let returns = cfg
            .returns
            .iter()
            .map(|p| Parameter::try_from(p))
            .collect::<Result<Vec<Parameter>, &'static str>>()?;

        let cfg = Cfg {
            name: cfg.name.clone(),
            function_no: cfg.function_no,
            params: Arc::new(params),
            returns: Arc::new(returns),
            vartable,
            blocks,
            nonpayable: cfg.nonpayable,
            public: cfg.public,
            ty: cfg.ty,
            selector: cfg.selector.clone(),
        };

        Ok(cfg)
    }
}
