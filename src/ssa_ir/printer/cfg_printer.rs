use std::io::Write;
use crate::codegen::cfg::ASTFunction;
use crate::ssa_ir::cfg::Cfg;
use crate::ssa_ir::printer::Printer;

#[macro_export]
macro_rules! stringfy_cfg {
    ($vartable:expr, $cfg:expr) => {{
        use solang::ssa_ir::printer::Printer;
        let mut printer = Printer { vartable: $vartable };
        let mut buf = Vec::new();
        printer.print_cfg(&mut buf, $cfg).unwrap();
        String::from_utf8(buf).unwrap()
    }}
}

impl Printer<'_> {
    pub fn print_cfg(&self, f: &mut dyn Write, cfg: &Cfg) -> std::io::Result<()> {
        let function_no = match cfg.function_no {
            ASTFunction::SolidityFunction(no) => format!("sol#{}", no),
            ASTFunction::YulFunction(no) => format!("yul#{}", no),
            ASTFunction::None => format!("none"),
        };

        let access_ctl = if cfg.public { "public" } else { "private" };

        write!(
            f,
            "{} {} {} {} ",
            access_ctl, cfg.ty, function_no, cfg.name
        )?;

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
        write!(f, ":\n")?;

        for (i, block) in cfg.blocks.iter().enumerate() {
            writeln!(f, "block#{} {}:", i, block.name)?;
            self.print_block(f, block)?;
        }
        Ok(())
    }
}