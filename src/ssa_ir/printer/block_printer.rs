use std::io::Write;
use crate::ssa_ir::cfg::Block;
use crate::ssa_ir::printer::Printer;

#[macro_export]
macro_rules! stringfy_block {
    ($vartable:expr, $block:expr) => {{
        use solang::ssa_ir::printer::Printer;
        let mut printer = Printer { vartable: $vartable };
        let mut buf = Vec::new();
        printer.print_block(&mut buf, $block).unwrap();
        String::from_utf8(buf).unwrap()
    }}
}

impl Printer<'_> {
    pub fn print_block(&self, f: &mut dyn Write, block: &Block) -> std::io::Result<()> {
        for insn in &block.instructions {
            write!(f, "    ")?;
            self.print_insn(f, insn)?;
            writeln!(f)?;
        }
        Ok(())
    }
}