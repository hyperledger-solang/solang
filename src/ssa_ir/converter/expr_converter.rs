use crate::codegen::Expression;
use crate::ssa_ir::converter::Converter;
use crate::ssa_ir::expr::Operand;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::vartable::Vartable;

impl Converter {
    pub(crate) fn from_expression(dest: &Operand, expr: &Expression, vartable: &mut Vartable) -> Result<Vec<Insn>, &'static str> {
        match expr {
            Expression::Add {
                loc, ty, overflowing, left, right
            } => todo!("Expression::Add"),
            Expression::AllocDynamicBytes { .. } => todo!("Expression::AllocDynamicBytes"),
            Expression::ArrayLiteral { .. } => todo!("Expression::ArrayLiteral"),
            Expression::BitwiseAnd { .. } => todo!("Expression::BitwiseAnd"),
            Expression::BitwiseOr { .. } => todo!("Expression::BitwiseOr"),
            Expression::BitwiseXor { .. } => todo!("Expression::BitwiseXor"),
            Expression::BoolLiteral { .. } => todo!("Expression::BoolLiteral"),
            Expression::Builtin { .. } => todo!("Expression::Builtin"),
            Expression::BytesCast { .. } => todo!("Expression::BytesCast"),
            Expression::BytesLiteral { .. } => todo!("Expression::BytesLiteral"),
            Expression::Cast { .. } => todo!("Expression::Cast"),
            Expression::BitwiseNot { .. } => todo!("Expression::BitwiseNot"),
            Expression::ConstArrayLiteral { .. } => todo!("Expression::ConstArrayLiteral"),
            Expression::UnsignedDivide { .. } => todo!("Expression::UnsignedDivide"),
            Expression::SignedDivide { .. } => todo!("Expression::SignedDivide"),
            Expression::Equal { .. } => todo!("Expression::Equal"),
            Expression::FormatString { .. } => todo!("Expression::FormatString"),
            Expression::FunctionArg { .. } => todo!("Expression::FunctionArg"),
            Expression::GetRef { .. } => todo!("Expression::GetRef"),
            Expression::InternalFunctionCfg { .. } => todo!("Expression::InternalFunctionCfg"),
            Expression::Keccak256 { .. } => todo!("Expression::Keccak256"),
            Expression::List { .. } => todo!("Expression::List"),
            Expression::Less { .. } => todo!("Expression::Less"),
            Expression::LessEqual { .. } => todo!("Expression::LessEqual"),
            Expression::Load { .. } => todo!("Expression::Load"),
            Expression::UnsignedModulo { .. } => todo!("Expression::UnsignedModulo"),
            Expression::SignedModulo { .. } => todo!("Expression::SignedModulo"),
            Expression::More { .. } => todo!("Expression::More"),
            Expression::MoreEqual { .. } => todo!("Expression::MoreEqual"),
            Expression::Multiply { .. } => todo!("Expression::Multiply"),
            Expression::Not { .. } => todo!("Expression::Not"),
            Expression::NotEqual { .. } => todo!("Expression::NotEqual"),
            Expression::NumberLiteral { .. } => todo!("Expression::NumberLiteral"),
            Expression::Poison => todo!("Expression::Poison"),
            Expression::Power { .. } => todo!("Expression::Power"),
            Expression::RationalNumberLiteral { .. } => todo!("Expression::RationalNumberLiteral"),
            Expression::ReturnData { .. } => todo!("Expression::ReturnData"),
            Expression::SignExt { .. } => todo!("Expression::SignExt"),
            Expression::ShiftLeft { .. } => todo!("Expression::ShiftLeft"),
            Expression::ShiftRight { .. } => todo!("Expression::ShiftRight"),
            Expression::StorageArrayLength { .. } => todo!("Expression::StorageArrayLength"),
            Expression::StringCompare { .. } => todo!("Expression::StringCompare"),
            Expression::StringConcat { .. } => todo!("Expression::StringConcat"),
            Expression::StructLiteral { .. } => todo!("Expression::StructLiteral"),
            Expression::StructMember { .. } => todo!("Expression::StructMember"),
            Expression::Subscript { .. } => todo!("Expression::Subscript"),
            Expression::Subtract { .. } => todo!("Expression::Subtract"),
            Expression::Trunc { .. } => todo!("Expression::Trunc"),
            Expression::Negate { .. } => todo!("Expression::Negate"),
            Expression::Undefined { .. } => todo!("Expression::Undefined"),
            Expression::Variable { .. } => todo!("Expression::Variable"),
            Expression::ZeroExt { .. } => todo!("Expression::ZeroExt"),
            Expression::AdvancePointer { .. } => todo!("Expression::AdvancePointer"),
        }
    }
}