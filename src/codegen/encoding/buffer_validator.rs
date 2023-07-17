// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::revert::{assert_failure, PanicCode, SolidityError};
use crate::codegen::vartable::Vartable;
use crate::codegen::Expression;
use crate::sema::ast::{Namespace, Type};
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::pt::Loc;
use std::ops::AddAssign;

/// When we are decoding serialized data from a bytes array, we must constantly verify if
/// we are not reading past its ending. This struct helps us decrease the number of checks we do,
/// by merging checks when we can determine the size of what to read beforehand.
pub(crate) struct BufferValidator<'a> {
    /// Saves the codegen::Expression that contains the buffer length.
    buffer_length: Expression,
    /// The types we are supposed to decode
    types: &'a [Type],
    /// The argument whose size has already been accounted for when verifying the buffer
    verified_until: Option<usize>,
    /// The argument we are analysing presently
    current_arg: usize,
}

impl BufferValidator<'_> {
    pub fn new(buffer_size_var: usize, types: &[Type]) -> BufferValidator {
        BufferValidator {
            buffer_length: Expression::Variable {
                loc: Loc::Codegen,
                ty: Type::Uint(32),
                var_no: buffer_size_var,
            },
            types,
            verified_until: None,
            current_arg: 0,
        }
    }

    /// Set which item we are currently reading from the buffer
    pub(super) fn set_argument_number(&mut self, arg_no: usize) {
        self.current_arg = arg_no;
    }

    /// Initialize the validator, by verifying every type that has a fixed size
    pub(super) fn initialize_validation(
        &mut self,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        // -1 means nothing has been verified yet
        self.verified_until = None;
        self._verify_buffer(offset, ns, vartab, cfg);
    }

    /// Validate the buffer for the current argument, if necessary.
    pub(super) fn validate_buffer(
        &mut self,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        // We may have already verified this
        if self.verified_until.is_some() && self.current_arg <= self.verified_until.unwrap() {
            return;
        }

        self._verify_buffer(offset, ns, vartab, cfg);
    }

    /// Validate if a given offset is within the buffer's bound.
    pub(super) fn validate_offset(
        &self,
        offset: Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        self.build_out_of_bounds_fail_branch(offset, ns, vartab, cfg);
    }

    /// Checks if a buffer validation is necessary
    pub(super) fn validation_necessary(&self) -> bool {
        self.verified_until.is_none() || self.current_arg > self.verified_until.unwrap()
    }

    /// After an array validation, we do not need to re-check its elements.
    pub(super) fn validate_array(&mut self) {
        self.verified_until = Some(self.current_arg);
    }

    /// Validate if offset + size is within the buffer's boundaries
    pub(super) fn validate_offset_plus_size(
        &mut self,
        offset: &Expression,
        size: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        if self.validation_necessary() {
            let offset_to_validate = Expression::Add {
                loc: Loc::Codegen,
                ty: Type::Uint(32),
                overflowing: false,
                left: Box::new(offset.clone()),
                right: Box::new(size.clone()),
            };
            self.validate_offset(offset_to_validate, ns, vartab, cfg);
        }
    }

    /// Validates if we have read all the bytes in a buffer
    pub(super) fn validate_all_bytes_read(
        &self,
        end_offset: Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        let cond = Expression::Less {
            loc: Loc::Codegen,
            signed: false,
            left: Box::new(end_offset),
            right: Box::new(self.buffer_length.clone()),
        };

        let invalid = cfg.new_basic_block("not_all_bytes_read".to_string());
        let valid = cfg.new_basic_block("buffer_read".to_string());
        cfg.add(
            vartab,
            Instr::BranchCond {
                cond,
                true_block: invalid,
                false_block: valid,
            },
        );

        cfg.set_basic_block(invalid);

        // TODO: This needs a proper error message
        let error = SolidityError::Panic(PanicCode::Generic);
        assert_failure(&Loc::Codegen, error, ns, cfg, vartab);

        cfg.set_basic_block(valid);
    }

    /// Auxiliary function to verify if the offset is valid.
    fn _verify_buffer(
        &mut self,
        offset: &Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        // Calculate the what arguments we can validate
        let mut maximum_verifiable = self.current_arg;
        for i in self.current_arg..self.types.len() {
            if !self.types[i].is_dynamic(ns) {
                maximum_verifiable = i;
            } else {
                break;
            }
        }

        // It is not possible to validate anything
        if maximum_verifiable == self.current_arg {
            return;
        }

        // Create validation check
        let mut advance = BigInt::zero();
        for i in self.current_arg..=maximum_verifiable {
            advance.add_assign(self.types[i].memory_size_of(ns));
        }

        let reach = Expression::Add {
            loc: Loc::Codegen,
            ty: Type::Uint(32),
            overflowing: false,
            left: Box::new(offset.clone()),
            right: Box::new(Expression::NumberLiteral {
                loc: Loc::Codegen,
                ty: Type::Uint(32),
                value: advance,
            }),
        };

        self.verified_until = Some(maximum_verifiable);
        self.build_out_of_bounds_fail_branch(reach, ns, vartab, cfg);
    }

    /// Builds a branch for failing if we are out of bounds
    fn build_out_of_bounds_fail_branch(
        &self,
        offset: Expression,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) {
        let cond = Expression::LessEqual {
            loc: Loc::Codegen,
            signed: false,
            left: Box::new(offset),
            right: Box::new(self.buffer_length.clone()),
        };

        let inbounds_block = cfg.new_basic_block("inbounds".to_string());
        let out_of_bounds_block = cfg.new_basic_block("out_of_bounds".to_string());

        cfg.add(
            vartab,
            Instr::BranchCond {
                cond,
                true_block: inbounds_block,
                false_block: out_of_bounds_block,
            },
        );

        cfg.set_basic_block(out_of_bounds_block);
        // TODO: Add an error message here
        let error = SolidityError::Panic(PanicCode::Generic);
        assert_failure(&Loc::Codegen, error, ns, cfg, vartab);
        cfg.set_basic_block(inbounds_block);
    }

    /// Create a new buffer validator to validate struct fields.
    pub(super) fn create_sub_validator<'a>(&self, types: &'a [Type]) -> BufferValidator<'a> {
        // If the struct has been previously validated, there is no need to validate it again,
        // so verified_until and current_arg are set to type.len() to avoid any further validation.
        BufferValidator {
            buffer_length: self.buffer_length.clone(),
            types,
            verified_until: if self.validation_necessary() {
                None
            } else {
                Some(types.len())
            },
            current_arg: if self.validation_necessary() {
                0
            } else {
                types.len()
            },
        }
    }
}
