use crate::codegen::cfg::ASTFunction;
use crate::pt::FunctionTy;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::ssa_type::Parameter;
use crate::ssa_ir::vartable::Vartable;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub struct Cfg {
    // FIXME: need some adjustments on the params and types
    pub name: String,
    pub function_no: ASTFunction,
    // TODO: define a new type for params?
    pub params: Arc<Vec<Parameter>>,
    pub returns: Arc<Vec<Parameter>>,
    pub vartable: Vartable,
    pub blocks: Vec<Block>,
    pub nonpayable: bool,
    pub public: bool,
    pub ty: FunctionTy,
    /// used to match the function in the contract
    pub selector: Vec<u8>,
}

#[derive(Debug)]
pub struct Block {
    pub name: String,
    pub instructions: Vec<Insn>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "block {}:\n", self.name)?;
        for insn in &self.instructions {
            write!(f, "    {}\n", insn)?;
        }
        Ok(())
    }
}

impl fmt::Display for Cfg {
    /// <public> <ty> function#<function_no> <name> (<params>) returns (<returns>):
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let function_no = match self.function_no {
            ASTFunction::SolidityFunction(no) => format!("sol#{}", no),
            ASTFunction::YulFunction(no) => format!("yul#{}", no),
            ASTFunction::None => format!("none"),
        };

        let access_ctl = if self.public { "public" } else { "private" };

        write!(
            f,
            "{} {} function#{} {} ",
            access_ctl, self.ty, function_no, self.name
        )?;

        write!(f, "(")?;
        for (i, param) in self.params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param.ty)?;
        }
        write!(f, ")")?;

        if !self.returns.is_empty() {
            write!(f, " returns (")?;
            for (i, ret) in self.returns.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", ret.ty)?;
            }
            write!(f, ")")?;
        }

        for block in &self.blocks {
            write!(f, "{}", block)?;
        }
        Ok(())
    }
}
