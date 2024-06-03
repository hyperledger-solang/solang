// SPDX-License-Identifier: Apache-2.0

mod arithmetic;
mod assign;
pub(crate) mod constructor;
pub(crate) mod function_call;
pub(crate) mod integers;
pub(crate) mod literals;
mod member_access;
pub mod resolve_expression;
pub mod retrieve_type;
pub(crate) mod strings;
mod subscript;
mod tests;
mod variable;

use super::ast::{ArrayLength, Diagnostic, Expression, Mutability, Namespace, RetrieveType, Type};
use super::diagnostics::Diagnostics;
use super::eval::eval_const_rational;
use super::symtable::{Symtable, VarScope};
use crate::sema::contracts::is_base;
use crate::sema::eval::eval_const_number;
use crate::sema::{symtable::LoopScopes, using::user_defined_operator_binding};
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use solang_parser::pt::{self, CodeLocation};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Compare two mutability levels
pub fn compatible_mutability(left: &Mutability, right: &Mutability) -> bool {
    matches!(
        (left, right),
        // only payable is compatible with payable
        (Mutability::Payable(_), Mutability::Payable(_))
            // default is compatible with anything but pure and view
            | (Mutability::Nonpayable(_), Mutability::Nonpayable(_) | Mutability::Payable(_))
            // view is compatible with anything but pure
            | (Mutability::View(_), Mutability::View(_) | Mutability::Nonpayable(_) | Mutability::Payable(_))
            // pure is compatible with anything
            | (Mutability::Pure(_), _) // everything else is not compatible
    )
}

/// When resolving an expression, what type are we looking for
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ResolveTo<'a> {
    Unknown,        // We don't know what we're looking for, best effort
    Integer,        // Try to resolve to an integer type value (signed or unsigned, any bit width)
    Discard,        // We won't be using the result. For example, an expression as a statement
    Type(&'a Type), // We will be wanting this type please, e.g. `int64 x = 1;`
}

#[derive(Default)]
pub struct ExprContext {
    /// What source file are we in
    pub file_no: usize,
    // Are we resolving a contract, and if so, which one
    pub contract_no: Option<usize>,
    /// Are resolving the body of a function, and if so, which one
    pub function_no: Option<usize>,
    /// Are we currently in an unchecked block
    pub unchecked: bool,
    /// Are we evaluating a constant expression
    pub constant: bool,
    /// Are we resolving an l-value
    pub lvalue: bool,
    /// Are we resolving a yul function (it cannot have external dependencies)
    pub yul_function: bool,
    /// Loops nesting
    pub loops: LoopScopes,
    /// Stack of currently active variable scopes
    pub active_scopes: Vec<VarScope>,
    /// Solidity v0.5 and earlier don't complain about emit resolving to multiple events
    pub ambiguous_emit: bool,
}

impl ExprContext {
    pub fn enter_scope(&mut self) {
        self.active_scopes.push(VarScope {
            loc: None,
            names: HashMap::new(),
        });
    }

    pub fn leave_scope(&mut self, symtable: &mut Symtable, loc: pt::Loc) {
        if let Some(mut curr_scope) = self.active_scopes.pop() {
            curr_scope.loc = Some(loc);
            symtable.scopes.push(curr_scope);
        }
    }
}

impl Expression {
    /// Is this expression 0
    pub(super) fn const_zero(&self, ns: &Namespace) -> bool {
        let mut diagnostics = Diagnostics::default();
        if let Ok((_, value)) = eval_const_number(self, ns, &mut diagnostics) {
            value == BigInt::zero()
        } else {
            false
        }
    }

    /// Return the type for this expression.
    pub fn tys(&self) -> Vec<Type> {
        match self {
            Expression::Builtin { tys: returns, .. }
            | Expression::InternalFunctionCall { returns, .. }
            | Expression::ExternalFunctionCall { returns, .. } => returns.to_vec(),
            Expression::List { list, .. } => list.iter().map(|e| e.ty()).collect(),
            Expression::ExternalFunctionCallRaw { .. } => vec![Type::Bool, Type::DynamicBytes],
            _ => vec![self.ty()],
        }
    }

    /// Cast from one type to another, which also automatically derefs any Type::Ref() type.
    /// if the cast is explicit (e.g. bytes32(bar) then implicit should be set to false.
    pub(crate) fn cast(
        &self,
        loc: &pt::Loc,
        to: &Type,
        implicit: bool,
        ns: &Namespace,
        diagnostics: &mut Diagnostics,
    ) -> Result<Expression, ()> {
        let from = self.ty();
        if &from == to {
            return Ok(self.clone());
        }

        if from == Type::Unresolved || *to == Type::Unresolved {
            return Ok(self.clone());
        }

        // First of all, if we have a ref then dereference it
        if let Type::Ref(r) = &from {
            return if r.is_fixed_reference_type(ns) {
                // A struct/fixed array *value* is simply the type, e.g. Type::Struct(_)
                // An assignable struct value, e.g. member of another struct, is Type::Ref(Type:Struct(_)).
                // However, the underlying types are identical: simply a pointer.
                //
                // So a Type::Ref(Type::Struct(_)) can be cast to Type::Struct(_).
                //
                // The Type::Ref(..) just means it can be used as an l-value and assigned
                // a new value, unlike say, a struct literal.
                if r.as_ref() == to {
                    Ok(self.clone())
                } else {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "conversion from {} to {} not possible",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                }
            } else {
                Expression::Load {
                    loc: *loc,
                    ty: r.as_ref().clone(),
                    expr: Box::new(self.clone()),
                }
                .cast(loc, to, implicit, ns, diagnostics)
            };
        }

        // If it's a storage reference then load the value. The expr is the storage slot
        if let Type::StorageRef(_, r) = from {
            if let Expression::Subscript { array_ty: ty, .. } = self {
                if ty.is_storage_bytes() {
                    return Ok(self.clone());
                }
            }

            return Expression::StorageLoad {
                loc: *loc,
                ty: *r,
                expr: Box::new(self.clone()),
            }
            .cast(loc, to, implicit, ns, diagnostics);
        }

        // Special case: when converting literal sign can change if it fits
        match (self, &from, to) {
            (Expression::NumberLiteral { value, .. }, p, &Type::Uint(to_len))
                if p.is_primitive() =>
            {
                return if value.sign() == Sign::Minus {
                    if implicit {
                        diagnostics.push(Diagnostic::cast_error(
                            *loc,
                            format!(
                                "implicit conversion cannot change negative number to '{}'",
                                to.to_string(ns)
                            ),
                        ));
                        Err(())
                    } else {
                        // Convert to little endian so most significant bytes are at the end; that way
                        // we can simply resize the vector to the right size
                        let mut bs = value.to_signed_bytes_le();

                        bs.resize(to_len as usize / 8, 0xff);
                        Ok(Expression::NumberLiteral {
                            loc: *loc,
                            ty: Type::Uint(to_len),
                            value: BigInt::from_bytes_le(Sign::Plus, &bs),
                        })
                    }
                } else if value.bits() >= to_len as u64 {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from '{}' to '{}'",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Uint(to_len),
                        value: value.clone(),
                    })
                };
            }
            (Expression::NumberLiteral { value, .. }, p, &Type::Int(to_len))
                if p.is_primitive() =>
            {
                return if value.bits() >= to_len as u64 {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from '{}' to '{}'",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Int(to_len),
                        value: value.clone(),
                    })
                };
            }
            (Expression::NumberLiteral { value, .. }, p, &Type::Bytes(to_len))
                if p.is_primitive() =>
            {
                return if value.sign() == Sign::Minus {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "negative number cannot be converted to type '{}'",
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if value.sign() == Sign::Plus && from.bytes(ns) != to_len {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "number of {} bytes cannot be converted to type '{}'",
                            from.bytes(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Bytes(to_len),
                        value: value.clone(),
                    })
                };
            }
            (Expression::NumberLiteral { value, .. }, p, &Type::Address(payable))
                if p.is_primitive() =>
            {
                // note: negative values are allowed
                return if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        String::from("implicit conversion from int to address not allowed"),
                    ));
                    Err(())
                } else if value.bits() > ns.address_length as u64 * 8 {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "number larger than possible in {} byte address",
                            ns.address_length,
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Address(payable),
                        value: value.clone(),
                    })
                };
            }
            // Literal strings can be implicitly lengthened
            (Expression::BytesLiteral { value, .. }, p, &Type::Bytes(to_len))
                if p.is_primitive() =>
            {
                return if value.len() > to_len as usize && implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from '{}' to '{}'",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    let mut bs = value.to_owned();

                    // Add zero's at the end as needed
                    bs.resize(to_len as usize, 0);

                    Ok(Expression::BytesLiteral {
                        loc: *loc,
                        ty: Type::Bytes(to_len),
                        value: bs,
                    })
                };
            }
            (Expression::BytesLiteral { loc, value, .. }, _, &Type::DynamicBytes)
            | (Expression::BytesLiteral { loc, value, .. }, _, &Type::String) => {
                return Ok(Expression::AllocDynamicBytes {
                    loc: *loc,
                    ty: to.clone(),
                    length: Box::new(Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Uint(32),
                        value: BigInt::from(value.len()),
                    }),
                    init: Some(value.clone()),
                });
            }
            (Expression::NumberLiteral { value, .. }, _, &Type::Rational) => {
                return Ok(Expression::RationalNumberLiteral {
                    loc: *loc,
                    ty: Type::Rational,
                    value: BigRational::from(value.clone()),
                });
            }

            (
                Expression::ArrayLiteral { .. },
                Type::Array(from_ty, from_dims),
                Type::Array(to_ty, to_dims),
            ) => {
                if from_ty == to_ty
                    && from_dims.len() == to_dims.len()
                    && from_dims.len() == 1
                    && matches!(to_dims.last().unwrap(), ArrayLength::Dynamic)
                {
                    return Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    });
                }
            }

            _ => (),
        };

        self.cast_types(loc, &from, to, implicit, ns, diagnostics)
    }

    fn cast_types(
        &self,
        loc: &pt::Loc,
        from: &Type,
        to: &Type,
        implicit: bool,
        ns: &Namespace,
        diagnostics: &mut Diagnostics,
    ) -> Result<Expression, ()> {
        let address_bits = ns.address_length as u16 * 8;

        match (&from, &to) {
            // Solana builtin AccountMeta struct wants a pointer to an address for the pubkey field,
            // not an address. For this specific field we have a special Expression::GetRef() which
            // gets the pointer to an address
            (Type::Address(_), Type::Ref(to)) if matches!(to.as_ref(), Type::Address(..)) => {
                Ok(Expression::GetRef {
                    loc: *loc,
                    ty: Type::Ref(Box::new(from.clone())),
                    expr: Box::new(self.clone()),
                })
            }
            (Type::Bytes(1), Type::Enum(_)) if !implicit => {
                self.cast_types(loc, &Type::Uint(8), to, implicit, ns, diagnostics)
            }
            (Type::Uint(from_width), Type::Enum(enum_no))
            | (Type::Int(from_width), Type::Enum(enum_no)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion from {} to {} not allowed",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    return Err(());
                }

                let enum_ty = &ns.enums[*enum_no];

                // TODO would be help to have current contract to resolve contract constants
                let mut temp_diagnostics = Diagnostics::default();

                if let Ok((_, big_number)) = eval_const_number(self, ns, &mut temp_diagnostics) {
                    if let Some(number) = big_number.to_usize() {
                        if enum_ty.values.len() > number {
                            return Ok(Expression::NumberLiteral {
                                loc: self.loc(),
                                ty: to.clone(),
                                value: big_number,
                            });
                        }
                    }

                    // solc does not detect this problem, just warn about it
                    diagnostics.push(Diagnostic::warning(
                        *loc,
                        format!(
                            "enum {} has no value with ordinal {}",
                            to.to_string(ns),
                            big_number
                        ),
                    ));
                }

                let to_width = enum_ty.ty.bits(ns);

                // TODO needs runtime checks
                match from_width.cmp(&to_width) {
                    Ordering::Greater => Ok(Expression::Trunc {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                    Ordering::Less => Ok(Expression::ZeroExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                    Ordering::Equal => Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                }
            }
            (Type::Enum(enum_no), Type::Uint(to_width))
            | (Type::Enum(enum_no), Type::Int(to_width)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion from {} to {} not allowed",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
                let enum_ty = &ns.enums[*enum_no];
                let from_width = enum_ty.ty.bits(ns);

                match from_width.cmp(to_width) {
                    Ordering::Greater => Ok(Expression::Trunc {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                    Ordering::Less => Ok(Expression::ZeroExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                    Ordering::Equal => Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                }
            }
            (Type::Bytes(n), Type::FunctionSelector) if *n == ns.target.selector_length() => {
                Ok(Expression::Cast {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                })
            }

            (Type::Bytes(1), Type::Uint(8)) | (Type::Uint(8), Type::Bytes(1)) => Ok(self.clone()),
            (Type::Uint(from_len), Type::Uint(to_len)) => match from_len.cmp(to_len) {
                Ordering::Greater => {
                    if implicit {
                        diagnostics.push(Diagnostic::cast_error(
                            *loc,
                            format!(
                                "implicit conversion would truncate from {} to {}",
                                from.to_string(ns),
                                to.to_string(ns)
                            ),
                        ));
                        Err(())
                    } else {
                        Ok(Expression::Trunc {
                            loc: *loc,
                            to: to.clone(),
                            expr: Box::new(self.clone()),
                        })
                    }
                }
                Ordering::Less => Ok(Expression::ZeroExt {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                }),
                Ordering::Equal => Ok(Expression::Cast {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                }),
            },
            (Type::Int(from_len), Type::Int(to_len)) => match from_len.cmp(to_len) {
                Ordering::Greater => {
                    if implicit {
                        diagnostics.push(Diagnostic::cast_error(
                            *loc,
                            format!(
                                "implicit conversion would truncate from {} to {}",
                                from.to_string(ns),
                                to.to_string(ns)
                            ),
                        ));
                        Err(())
                    } else {
                        Ok(Expression::Trunc {
                            loc: *loc,
                            to: to.clone(),
                            expr: Box::new(self.clone()),
                        })
                    }
                }
                Ordering::Less => Ok(Expression::SignExt {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                }),
                Ordering::Equal => Ok(Expression::Cast {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                }),
            },
            (Type::Uint(from_len), Type::Int(to_len)) if to_len > from_len => {
                Ok(Expression::ZeroExt {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                })
            }
            (Type::Int(from_len), Type::Uint(to_len)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would change sign from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if from_len > to_len {
                    Ok(Expression::Trunc {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                } else if from_len < to_len {
                    Ok(Expression::SignExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            (Type::Uint(from_len), Type::Int(to_len)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would change sign from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if from_len > to_len {
                    Ok(Expression::Trunc {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                } else if from_len < to_len {
                    Ok(Expression::ZeroExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Casting value to uint
            (Type::Value, Type::Uint(to_len)) => {
                let from_len = ns.value_length * 8;
                let to_len = *to_len as usize;

                match from_len.cmp(&to_len) {
                    Ordering::Greater => {
                        if implicit {
                            diagnostics.push(Diagnostic::cast_error(
                                *loc,
                                format!(
                                    "implicit conversion would truncate from {} to {}",
                                    from.to_string(ns),
                                    to.to_string(ns)
                                ),
                            ));
                            Err(())
                        } else {
                            Ok(Expression::Trunc {
                                loc: *loc,
                                to: to.clone(),
                                expr: Box::new(self.clone()),
                            })
                        }
                    }
                    Ordering::Less => Ok(Expression::SignExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                    Ordering::Equal => Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                }
            }
            (Type::Value, Type::Int(to_len)) => {
                let from_len = ns.value_length * 8;
                let to_len = *to_len as usize;

                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would change sign from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if from_len > to_len {
                    Ok(Expression::Trunc {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                } else if from_len < to_len {
                    Ok(Expression::ZeroExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Casting value to uint
            (Type::Uint(from_len), Type::Value) => {
                let from_len = *from_len as usize;
                let to_len = ns.value_length * 8;

                match from_len.cmp(&to_len) {
                    Ordering::Greater => {
                        diagnostics.push(Diagnostic::cast_warning(
                            *loc,
                            format!(
                                "conversion truncates {} to {}, as value is type {} on target {}",
                                from.to_string(ns),
                                to.to_string(ns),
                                Type::Value.to_string(ns),
                                ns.target
                            ),
                        ));

                        Ok(Expression::CheckingTrunc {
                            loc: *loc,
                            to: to.clone(),
                            expr: Box::new(self.clone()),
                        })
                    }
                    Ordering::Less => Ok(Expression::SignExt {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                    Ordering::Equal => Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    }),
                }
            }
            // Casting int to address
            (Type::Uint(from_len), Type::Address(_)) | (Type::Int(from_len), Type::Address(_)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion from {} to address not allowed",
                            from.to_string(ns)
                        ),
                    ));

                    Err(())
                } else {
                    // cast integer it to integer of the same size of address with sign ext etc
                    let address_to_int = if from.is_signed_int(ns) {
                        Type::Int(address_bits)
                    } else {
                        Type::Uint(address_bits)
                    };

                    let expr = match from_len.cmp(&address_bits) {
                        Ordering::Greater => Expression::Trunc {
                            loc: *loc,
                            to: address_to_int,
                            expr: Box::new(self.clone()),
                        },
                        Ordering::Less if from.is_signed_int(ns) => Expression::ZeroExt {
                            loc: *loc,
                            to: address_to_int,
                            expr: Box::new(self.clone()),
                        },
                        Ordering::Less => Expression::SignExt {
                            loc: *loc,
                            to: address_to_int,
                            expr: Box::new(self.clone()),
                        },
                        Ordering::Equal => self.clone(),
                    };

                    // Now cast integer to address
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(expr),
                    })
                }
            }
            // Casting address to int
            (Type::Address(_), Type::Uint(to_len)) | (Type::Address(_), Type::Int(to_len)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion to {} from {} not allowed",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));

                    Err(())
                } else {
                    // first convert address to int/uint
                    let address_to_int = if to.is_signed_int(ns) {
                        Type::Int(address_bits)
                    } else {
                        Type::Uint(address_bits)
                    };

                    let expr = Expression::Cast {
                        loc: *loc,
                        to: address_to_int,
                        expr: Box::new(self.clone()),
                    };
                    // now resize int to request size with sign extension etc
                    match to_len.cmp(&address_bits) {
                        Ordering::Less => Ok(Expression::Trunc {
                            loc: *loc,
                            to: to.clone(),
                            expr: Box::new(expr),
                        }),
                        Ordering::Greater if to.is_signed_int(ns) => Ok(Expression::ZeroExt {
                            loc: *loc,
                            to: to.clone(),
                            expr: Box::new(expr),
                        }),
                        Ordering::Greater => Ok(Expression::SignExt {
                            loc: *loc,
                            to: to.clone(),
                            expr: Box::new(expr),
                        }),
                        Ordering::Equal => Ok(expr),
                    }
                }
            }
            // Lengthing or shorting a fixed bytes array
            (Type::Bytes(from_len), Type::Bytes(to_len)) => {
                if to_len > from_len {
                    let shift = (to_len - from_len) * 8;

                    Ok(Expression::ShiftLeft {
                        loc: *loc,
                        ty: to.clone(),
                        left: Box::new(Expression::ZeroExt {
                            loc: self.loc(),
                            to: to.clone(),
                            expr: Box::new(self.clone()),
                        }),
                        right: Box::new(Expression::NumberLiteral {
                            loc: *loc,
                            ty: Type::Uint(*to_len as u16 * 8),
                            value: BigInt::from_u8(shift).unwrap(),
                        }),
                    })
                } else if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    let shift = (from_len - to_len) * 8;

                    Ok(Expression::Trunc {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(Expression::ShiftRight {
                            loc: self.loc(),
                            ty: from.clone(),
                            left: Box::new(self.clone()),
                            right: Box::new(Expression::NumberLiteral {
                                loc: self.loc(),
                                ty: Type::Uint(*from_len as u16 * 8),
                                value: BigInt::from_u8(shift).unwrap(),
                            }),
                            sign: false,
                        }),
                    })
                }
            }
            (Type::Rational, Type::Uint(_) | Type::Int(_) | Type::Value) => {
                match eval_const_rational(self, ns) {
                    Ok((_, big_number)) => {
                        if big_number.is_integer() {
                            return Ok(Expression::Cast {
                                loc: *loc,
                                to: to.clone(),
                                expr: Box::new(self.clone()),
                            });
                        }

                        diagnostics.push(Diagnostic::cast_error(
                            *loc,
                            format!(
                                "conversion to {} from {} not allowed",
                                to.to_string(ns),
                                from.to_string(ns)
                            ),
                        ));

                        Err(())
                    }
                    Err(diag) => {
                        diagnostics.push(diag);
                        Err(())
                    }
                }
            }
            (Type::Uint(_) | Type::Int(_) | Type::Value, Type::Rational) => Ok(Expression::Cast {
                loc: *loc,
                to: to.clone(),
                expr: Box::new(self.clone()),
            }),
            (Type::Bytes(_), Type::DynamicBytes) | (Type::DynamicBytes, Type::Bytes(_)) => {
                Ok(Expression::BytesCast {
                    loc: *loc,
                    to: to.clone(),
                    from: from.clone(),
                    expr: Box::new(self.clone()),
                })
            }
            // Explicit conversion from bytesN to int/uint only allowed with expliciy
            // cast and if it is the same size (i.e. no conversion required)
            (Type::Bytes(from_len), Type::Uint(to_len))
            | (Type::Bytes(from_len), Type::Int(to_len)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if *from_len as u16 * 8 != *to_len {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Explicit conversion to bytesN from int/uint only allowed with expliciy
            // cast and if it is the same size (i.e. no conversion required)
            (Type::Uint(from_len), Type::Bytes(to_len))
            | (Type::Int(from_len), Type::Bytes(to_len)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if *to_len as u16 * 8 != *from_len {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Explicit conversion from bytesN to address only allowed with expliciy
            // cast and if it is the same size (i.e. no conversion required)
            (Type::Bytes(from_len), Type::Address(_)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if *from_len as usize != ns.address_length {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Explicit conversion between contract and address is allowed
            (Type::Address(false), Type::Address(true))
            | (Type::Address(_), Type::Contract(_))
            | (Type::Contract(_), Type::Address(_)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Conversion between contracts is allowed if it is a base
            (Type::Contract(contract_no_from), Type::Contract(contract_no_to)) => {
                if implicit && !is_base(*contract_no_to, *contract_no_from, ns) {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion not allowed since {} is not a base contract of {}",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // conversion from address payable to address is implicitly allowed (not vice versa)
            (Type::Address(true), Type::Address(false)) => Ok(Expression::Cast {
                loc: *loc,
                to: to.clone(),
                expr: Box::new(self.clone()),
            }),
            // Explicit conversion to bytesN from int/uint only allowed with expliciy
            // cast and if it is the same size (i.e. no conversion required)
            (Type::Address(_), Type::Bytes(to_len)) => {
                if implicit {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "implicit conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if *to_len as usize != ns.address_length {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            (Type::String, Type::DynamicBytes) | (Type::DynamicBytes, Type::String)
                if !implicit =>
            {
                Ok(Expression::Cast {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                })
            }
            // string conversions
            // (Type::Bytes(_), Type::String) => Ok(Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))),
            /*
            (Type::String, Type::Bytes(to_len)) => {
                if let Expression::BytesLiteral(_, from_str) = self {
                    if from_str.len() > to_len as usize {
                        diagnostics.push(Output::type_error(
                            self.loc(),
                            format!(
                                "string of {} bytes is too long to fit into {}",
                                from_str.len(),
                                to.to_string(ns)
                            ),
                        ));
                        return Err(());
                    }
                }
                Ok(Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))
            }
            */
            (Type::Void, _) => {
                diagnostics.push(Diagnostic::cast_error(
                    self.loc(),
                    "function or method does not return a value".to_string(),
                ));
                Err(())
            }
            (
                Type::ExternalFunction {
                    params: from_params,
                    mutability: from_mutablity,
                    returns: from_returns,
                },
                Type::ExternalFunction {
                    params: to_params,
                    mutability: to_mutablity,
                    returns: to_returns,
                },
            )
            | (
                Type::InternalFunction {
                    params: from_params,
                    mutability: from_mutablity,
                    returns: from_returns,
                },
                Type::InternalFunction {
                    params: to_params,
                    mutability: to_mutablity,
                    returns: to_returns,
                },
            ) => {
                if from_params != to_params {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "function arguments do not match in conversion from '{}' to '{}'",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if from_returns != to_returns {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "function returns do not match in conversion from '{}' to '{}'",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));
                    Err(())
                } else if !compatible_mutability(from_mutablity, to_mutablity) {
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "function mutability not compatible in conversion from '{}' to '{}'",
                            from.to_string(ns),
                            to.to_string(ns),
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: Box::new(self.clone()),
                    })
                }
            }
            // Match any array with ArrayLength::AnyFixed if is it fixed for that dimension, and the
            // element type and other dimensions also match
            (Type::Array(from_elem, from_dim), Type::Array(to_elem, to_dim))
                if from_elem == to_elem
                    && from_dim.len() == to_dim.len()
                    && from_dim.iter().zip(to_dim.iter()).all(|(f, t)| {
                        f == t || matches!((f, t), (ArrayLength::Fixed(_), ArrayLength::AnyFixed))
                    }) =>
            {
                Ok(self.clone())
            }
            // bytes/address/bytesN -> slice bytes1
            (_, Type::Slice(ty)) if can_cast_to_slice(from) && ty.as_ref() == &Type::Bytes(1) => {
                Ok(Expression::Cast {
                    loc: *loc,
                    to: to.clone(),
                    expr: Box::new(self.clone()),
                })
            }
            // bytes[] -> slice slice bytes1
            (Type::Array(from, dims), Type::Slice(to))
                if dims.len() == 1
                    && (from == to
                        || (can_cast_to_slice(from)
                            && to.slice_depth() == (1, &Type::Bytes(1)))) =>
            {
                Ok(self.clone())
            }
            // bytes[][] -> slice slice slice bytes1
            (Type::Array(from, dims), Type::Slice(to))
                if dims.len() == 2
                    && (can_cast_to_slice(from) && to.slice_depth() == (2, &Type::Bytes(1))) =>
            {
                Ok(self.clone())
            }
            (Type::FunctionSelector, Type::Bytes(n)) => {
                let selector_length = ns.target.selector_length();
                if *n == selector_length {
                    Ok(Expression::Cast {
                        loc: *loc,
                        to: to.clone(),
                        expr: self.clone().into(),
                    })
                } else {
                    if *n < selector_length {
                        diagnostics.push(Diagnostic::warning(
                            *loc,
                            format!(
                                "function selector should only be casted to bytes{selector_length} or larger"
                            ),
                        ));
                    }
                    self.cast_types(
                        loc,
                        &Type::Bytes(selector_length),
                        to,
                        implicit,
                        ns,
                        diagnostics,
                    )
                }
            }
            (Type::FunctionSelector, Type::Uint(n) | Type::Int(n)) => {
                let selector_width = ns.target.selector_length() * 8;
                if *n < selector_width as u16 {
                    diagnostics.push(Diagnostic::warning(
                        *loc,
                        format!(
                            "function selector needs an integer of at least {selector_width} bits to avoid being truncated"
                        ),
                    ));
                }
                self.cast_types(
                    loc,
                    &Type::Bytes(ns.target.selector_length()),
                    to,
                    implicit,
                    ns,
                    diagnostics,
                )
            }
            _ => {
                diagnostics.push(Diagnostic::cast_error(
                    *loc,
                    format!(
                        "conversion from {} to {} not possible",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            }
        }
    }
}

/// Can this type be cast to a bytes slice
fn can_cast_to_slice(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Address(_) | Type::Bytes(_) | Type::DynamicBytes | Type::String
    )
}

/// Resolve operator with the given arguments to an expression, if possible
pub(super) fn user_defined_operator(
    loc: &pt::Loc,
    args: &[&Expression],
    oper: pt::UserDefinedOperator,
    diagnostics: &mut Diagnostics,
    ns: &Namespace,
) -> Option<Expression> {
    let ty = args[0].ty();
    let ty = ty.deref_any();

    if let Type::UserType(..) = ty {
        if let Some(using_function) = user_defined_operator_binding(ty, oper, ns) {
            if args.iter().all(|expr| expr.ty().deref_any() == ty) {
                let func = &ns.functions[using_function.function_no];

                return Some(Expression::UserDefinedOperator {
                    loc: *loc,
                    ty: func.returns[0].ty.clone(),
                    oper,
                    function_no: using_function.function_no,
                    args: args
                        .iter()
                        .map(|e| e.cast(&e.loc(), ty, true, ns, diagnostics).unwrap())
                        .collect(),
                });
            }
        }
    }

    None
}
