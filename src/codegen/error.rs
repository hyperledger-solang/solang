// SPDX-License-Identifier: Apache-2.0

use crate::Target;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt::Loc;
use std::fmt;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CodegenError {
    UnsupportedSorobanType {
        loc: Loc,
        operation: String,
        ty: String,
    },
    UnsupportedSorobanOperation {
        loc: Loc,
        operation: String,
    },
    MissingRuntimeHelper {
        helper: String,
        operation: String,
        target: Target,
    },
    LlvmBuilder {
        operation: String,
        error: String,
    },
    MissingLlvmEntity {
        operation: String,
        entity: String,
    },
    InvalidCfgInvariant {
        operation: String,
        reason: String,
    },
    NumericConversion {
        operation: String,
        value: String,
        target_type: String,
    },
}

#[allow(dead_code)]
impl CodegenError {
    pub fn unsupported_soroban_type(loc: Loc, operation: impl Into<String>, ty: String) -> Self {
        Self::UnsupportedSorobanType {
            loc,
            operation: operation.into(),
            ty,
        }
    }

    pub fn unsupported_soroban_operation(loc: Loc, operation: impl Into<String>) -> Self {
        Self::UnsupportedSorobanOperation {
            loc,
            operation: operation.into(),
        }
    }

    pub fn missing_runtime_helper(
        helper: impl Into<String>,
        operation: impl Into<String>,
        target: Target,
    ) -> Self {
        Self::MissingRuntimeHelper {
            helper: helper.into(),
            operation: operation.into(),
            target,
        }
    }

    #[cfg(feature = "llvm")]
    pub fn llvm_builder(
        operation: impl Into<String>,
        error: inkwell::builder::BuilderError,
    ) -> Self {
        let error = match error {
            inkwell::builder::BuilderError::UnsetPosition => {
                "builder had no insertion point".to_string()
            }
            inkwell::builder::BuilderError::AlignmentError(reason) => {
                format!("invalid LLVM alignment: {reason}")
            }
            inkwell::builder::BuilderError::ExtractOutOfRange => {
                "aggregate index out of range".to_string()
            }
            inkwell::builder::BuilderError::BitwidthError(reason) => {
                format!("invalid integer bit width: {reason}")
            }
            inkwell::builder::BuilderError::PointeeTypeMismatch(reason) => {
                format!("pointer/value type mismatch: {reason}")
            }
            inkwell::builder::BuilderError::ValueTypeMismatch(reason) => {
                format!("LLVM value type mismatch: {reason}")
            }
            inkwell::builder::BuilderError::OrderingError(reason) => {
                format!("invalid atomic ordering: {reason}")
            }
            inkwell::builder::BuilderError::GEPPointee => {
                "GEP expected a struct pointee".to_string()
            }
            inkwell::builder::BuilderError::GEPIndex => "GEP struct index out of range".to_string(),
        };

        Self::LlvmBuilder {
            operation: operation.into(),
            error,
        }
    }

    pub fn missing_llvm_entity(operation: impl Into<String>, entity: impl Into<String>) -> Self {
        Self::MissingLlvmEntity {
            operation: operation.into(),
            entity: entity.into(),
        }
    }

    pub fn invalid_cfg_invariant(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidCfgInvariant {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    pub fn numeric_conversion(
        operation: impl Into<String>,
        value: impl Into<String>,
        target_type: impl Into<String>,
    ) -> Self {
        Self::NumericConversion {
            operation: operation.into(),
            value: value.into(),
            target_type: target_type.into(),
        }
    }

    pub fn diagnostic(&self) -> Option<Diagnostic> {
        match self {
            Self::UnsupportedSorobanType { loc, operation, ty } => Some(Diagnostic::error(
                *loc,
                format!("type '{ty}' is not supported {operation} for target soroban"),
            )),
            Self::UnsupportedSorobanOperation { loc, operation } => Some(Diagnostic::error(
                *loc,
                format!("{operation} is not supported for target soroban"),
            )),
            _ => None,
        }
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSorobanType { operation, ty, .. } => {
                write!(f, "type '{ty}' is not supported {operation} for target soroban")
            }
            Self::UnsupportedSorobanOperation { operation, .. } => {
                write!(f, "{operation} is not supported for target soroban")
            }
            Self::MissingRuntimeHelper {
                helper,
                operation,
                target,
            } => write!(
                f,
                "runtime helper `{helper}` is missing while emitting {operation} for target {target}; this operation is unsupported on this target without that helper"
            ),
            Self::LlvmBuilder { operation, error } => {
                write!(f, "internal compiler error while {operation}: {error}")
            }
            Self::MissingLlvmEntity { operation, entity } => {
                write!(
                    f,
                    "internal compiler error while {operation}: missing LLVM entity `{entity}`"
                )
            }
            Self::InvalidCfgInvariant { operation, reason } => {
                write!(
                    f,
                    "internal compiler error while {operation}: invalid CFG invariant: {reason}"
                )
            }
            Self::NumericConversion {
                operation,
                value,
                target_type,
            } => {
                write!(
                    f,
                    "internal compiler error while {operation}: cannot convert `{value}` to {target_type}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodegenError;
    use crate::Target;
    use solang_parser::pt::Loc;

    #[test]
    fn unsupported_soroban_type_display_is_user_facing() {
        let err = CodegenError::unsupported_soroban_type(
            Loc::Codegen,
            "by the Soroban encoder",
            "bytes32".to_string(),
        );

        assert_eq!(
            err.to_string(),
            "type 'bytes32' is not supported by the Soroban encoder for target soroban"
        );
    }

    #[test]
    fn missing_runtime_helper_display_names_target_and_operation() {
        let err = CodegenError::missing_runtime_helper(
            "__memcpy",
            "copying concat argument",
            Target::Soroban,
        );

        assert_eq!(
            err.to_string(),
            "runtime helper `__memcpy` is missing while emitting copying concat argument for target Soroban; this operation is unsupported on this target without that helper"
        );
    }

    #[cfg(feature = "llvm")]
    #[test]
    fn llvm_builder_errors_are_mapped_to_human_readable_messages() {
        use inkwell::builder::BuilderError;

        let cases = [
            (
                BuilderError::UnsetPosition,
                "builder had no insertion point",
            ),
            (
                BuilderError::AlignmentError("bad align"),
                "invalid LLVM alignment: bad align",
            ),
            (
                BuilderError::ExtractOutOfRange,
                "aggregate index out of range",
            ),
            (
                BuilderError::BitwidthError("bad width"),
                "invalid integer bit width: bad width",
            ),
            (
                BuilderError::PointeeTypeMismatch("bad pointee"),
                "pointer/value type mismatch: bad pointee",
            ),
            (
                BuilderError::ValueTypeMismatch("bad value"),
                "LLVM value type mismatch: bad value",
            ),
            (
                BuilderError::OrderingError("bad ordering"),
                "invalid atomic ordering: bad ordering",
            ),
            (BuilderError::GEPPointee, "GEP expected a struct pointee"),
            (BuilderError::GEPIndex, "GEP struct index out of range"),
        ];

        for (builder_error, expected) in cases {
            assert_eq!(
                CodegenError::llvm_builder("testing builder error mapping", builder_error)
                    .to_string(),
                format!("internal compiler error while testing builder error mapping: {expected}")
            );
        }
    }
}
