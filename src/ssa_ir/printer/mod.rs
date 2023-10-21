use crate::ssa_ir::vartable::Vartable;

pub mod cfg_printer;
pub mod insn_printer;
pub mod expr_printer;
pub mod block_printer;

pub struct Printer<'v> {
    pub vartable: &'v Vartable,
}