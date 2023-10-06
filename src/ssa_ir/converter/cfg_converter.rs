use crate::codegen::cfg::ControlFlowGraph;
use crate::ssa_ir::cfg::{Cfg, Block};
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::vartable::Vartable;

impl Converter {
    pub fn from_control_flow_graph(cfg: &ControlFlowGraph) -> Result<Cfg, &'static str> {
        let mut vartable = Vartable::from(&cfg.vars);

        let blocks = cfg.blocks.iter()
            .map(|block| Converter::from_basic_block(block, &mut vartable))
            .collect::<Result<Vec<Block>, &'static str>>()?;

        let cfg = Cfg {
            name: cfg.name.clone(),
            function_no: cfg.function_no,
            params: cfg.params.clone(),
            returns: cfg.returns.clone(),
            vartable,
            blocks,
            nonpayable: cfg.nonpayable,
            public: cfg.public,
            ty: cfg.ty,
            selector: cfg.selector.clone()
        };

        Ok(cfg)
    }
}