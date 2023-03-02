// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Expression, RetrieveType, Type};

impl RetrieveType for Expression {
    fn ty(&self) -> Type {
        match self {
            Expression::BoolLiteral { .. }
            | Expression::More { .. }
            | Expression::Less { .. }
            | Expression::MoreEqual { .. }
            | Expression::LessEqual { .. }
            | Expression::Equal { .. }
            | Expression::Or { .. }
            | Expression::And { .. }
            | Expression::NotEqual { .. }
            | Expression::Not { .. }
            | Expression::StringCompare { .. } => Type::Bool,
            Expression::CodeLiteral { .. } => Type::DynamicBytes,
            Expression::StringConcat { ty, .. }
            | Expression::BytesLiteral { ty, .. }
            | Expression::NumberLiteral { ty, .. }
            | Expression::RationalNumberLiteral { ty, .. }
            | Expression::StructLiteral { ty, .. }
            | Expression::ArrayLiteral { ty, .. }
            | Expression::ConstArrayLiteral { ty, .. }
            | Expression::Add { ty, .. }
            | Expression::Subtract { ty, .. }
            | Expression::Multiply { ty, .. }
            | Expression::Divide { ty, .. }
            | Expression::Modulo { ty, .. }
            | Expression::Power { ty, .. }
            | Expression::BitwiseOr { ty, .. }
            | Expression::BitwiseAnd { ty, .. }
            | Expression::BitwiseXor { ty, .. }
            | Expression::ShiftLeft { ty, .. }
            | Expression::ShiftRight { ty, .. }
            | Expression::Variable { ty, .. }
            | Expression::ConstantVariable { ty, .. }
            | Expression::StorageVariable { ty, .. }
            | Expression::Load { ty, .. }
            | Expression::GetRef { ty, .. }
            | Expression::StorageLoad { ty, .. }
            | Expression::Complement { ty, .. }
            | Expression::Negate { ty, .. }
            | Expression::ConditionalOperator { ty, .. }
            | Expression::StructMember { ty, .. }
            | Expression::AllocDynamicBytes { ty, .. }
            | Expression::PreIncrement { ty, .. }
            | Expression::PreDecrement { ty, .. }
            | Expression::PostIncrement { ty, .. }
            | Expression::PostDecrement { ty, .. }
            | Expression::Assign { ty, .. } => ty.clone(),
            Expression::Subscript { ty, .. } => ty.clone(),
            Expression::ZeroExt { to, .. }
            | Expression::SignExt { to, .. }
            | Expression::Trunc { to, .. }
            | Expression::CheckingTrunc { to, .. }
            | Expression::Cast { to, .. }
            | Expression::BytesCast { to, .. } => to.clone(),
            Expression::StorageArrayLength { ty, .. } => ty.clone(),
            Expression::ExternalFunctionCallRaw { .. } => {
                panic!("two return values");
            }
            Expression::Builtin { tys: returns, .. }
            | Expression::InternalFunctionCall { returns, .. }
            | Expression::ExternalFunctionCall { returns, .. } => {
                assert_eq!(returns.len(), 1);
                returns[0].clone()
            }
            Expression::List { list, .. } => {
                assert_eq!(list.len(), 1);

                list[0].ty()
            }
            Expression::Constructor { contract_no, .. } => Type::Contract(*contract_no),
            Expression::InterfaceId { .. } => Type::FunctionSelector,
            Expression::FormatString { .. } => Type::String,
            Expression::UserDefinedOperator { ty, .. }
            | Expression::InternalFunction { ty, .. }
            | Expression::ExternalFunction { ty, .. } => ty.clone(),
        }
    }
}
