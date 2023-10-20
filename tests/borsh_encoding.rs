// SPDX-License-Identifier: Apache-2.0

use anchor_syn::idl::types::{IdlType, IdlTypeDefinition, IdlTypeDefinitionTy};
use byte_slice_cast::AsByteSlice;
use num_bigint::{BigInt, Sign};
use num_traits::ToPrimitive;
use serde::Deserialize;
use std::cmp::Ordering;

/// This is the token that should be used for each function call in Solana runtime tests
#[derive(Debug, PartialEq, Clone, Deserialize)]
pub enum BorshToken {
    Address([u8; 32]),
    Int { width: u16, value: BigInt },
    Uint { width: u16, value: BigInt },
    FixedBytes(Vec<u8>),
    Bytes(Vec<u8>),
    Bool(bool),
    String(String),
    FixedArray(Vec<BorshToken>),
    Array(Vec<BorshToken>),
    Tuple(Vec<BorshToken>),
}

impl BorshToken {
    /// Encode the parameter into the buffer
    pub fn encode(&self, buffer: &mut Vec<u8>) {
        match self {
            BorshToken::Address(data) => {
                buffer.extend_from_slice(data);
            }
            BorshToken::Uint { width, value } => {
                encode_uint(*width, value, buffer);
            }
            BorshToken::Int { width, value } => {
                encode_int(*width, value, buffer);
            }
            BorshToken::FixedBytes(data) => {
                buffer.extend_from_slice(data);
            }
            BorshToken::Bytes(data) => {
                let len = data.len() as u32;
                buffer.extend_from_slice(&len.to_le_bytes());
                buffer.extend_from_slice(data);
            }
            BorshToken::Bool(value) => {
                buffer.push(*value as u8);
            }
            BorshToken::String(data) => {
                let len = data.len() as u32;
                buffer.extend_from_slice(&len.to_le_bytes());
                buffer.extend_from_slice(data.as_byte_slice());
            }
            BorshToken::Tuple(data) | BorshToken::FixedArray(data) => {
                for item in data {
                    item.encode(buffer);
                }
            }
            BorshToken::Array(arr) => {
                let len = arr.len() as u32;
                buffer.extend_from_slice(&len.to_le_bytes());
                for item in arr {
                    item.encode(buffer);
                }
            }
        }
    }

    pub fn into_string(self) -> Option<String> {
        match self {
            BorshToken::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn into_array(self) -> Option<Vec<BorshToken>> {
        match self {
            BorshToken::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn into_fixed_bytes(self) -> Option<Vec<u8>> {
        match self {
            BorshToken::FixedBytes(value) => Some(value),
            BorshToken::FixedArray(vec) => {
                let mut response: Vec<u8> = Vec::with_capacity(vec.len());
                for elem in vec {
                    match elem {
                        BorshToken::Uint { width, value } => {
                            assert_eq!(width, 8);
                            response.push(value.to_u8().unwrap());
                        }
                        _ => unreachable!("Array cannot be converted to fixed bytes"),
                    }
                }
                Some(response)
            }
            BorshToken::Address(value) => Some(value.to_vec()),
            _ => None,
        }
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            BorshToken::Bytes(value) => Some(value),
            _ => None,
        }
    }

    pub fn into_bigint(self) -> Option<BigInt> {
        match self {
            BorshToken::Uint { value, .. } => Some(value),
            BorshToken::Int { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn unwrap_tuple(self) -> Vec<BorshToken> {
        match self {
            BorshToken::Tuple(vec) => vec,
            _ => panic!("This is not a tuple"),
        }
    }

    pub fn uint8_fixed_array(vec: Vec<u8>) -> BorshToken {
        let mut array: Vec<BorshToken> = Vec::with_capacity(vec.len());
        for item in &vec {
            array.push(BorshToken::Uint {
                width: 8,
                value: BigInt::from(*item),
            });
        }

        BorshToken::FixedArray(array)
    }
}

/// Encode a signed integer
fn encode_int(width: u16, value: &BigInt, buffer: &mut Vec<u8>) {
    match width {
        1..=8 => {
            let val = value.to_i8().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        9..=16 => {
            let val = value.to_i16().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        17..=32 => {
            let val = value.to_i32().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        33..=64 => {
            let val = value.to_i64().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        65..=128 => {
            let val = value.to_i128().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        129..=256 => {
            let mut val = value.to_signed_bytes_le();
            let byte_width = 32;
            match val.len().cmp(&byte_width) {
                Ordering::Greater => {
                    while val.len() > byte_width {
                        val.pop();
                    }
                }
                Ordering::Less => {
                    if value.sign() == Sign::Minus {
                        val.extend(vec![255; byte_width - val.len()]);
                    } else {
                        val.extend(vec![0; byte_width - val.len()]);
                    }
                }

                Ordering::Equal => (),
            }

            buffer.extend_from_slice(&val);
        }
        _ => unreachable!("bit width not supported"),
    }
}

/// Encode an unsigned integer
fn encode_uint(width: u16, value: &BigInt, buffer: &mut Vec<u8>) {
    match width {
        1..=8 => {
            let val = value.to_u8().unwrap();
            buffer.push(val);
        }
        9..=16 => {
            let val = value.to_u16().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        17..=32 => {
            let val = value.to_u32().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        33..=64 => {
            let val = value.to_u64().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        65..=128 => {
            let val = value.to_u128().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        129..=256 => {
            let mut val = value.to_signed_bytes_le();
            let bytes_width = 32;
            match val.len().cmp(&bytes_width) {
                Ordering::Greater => {
                    while val.len() > bytes_width {
                        val.pop();
                    }
                }
                Ordering::Less => {
                    val.extend(vec![0; bytes_width - val.len()]);
                }

                Ordering::Equal => (),
            }

            buffer.extend_from_slice(&val);
        }

        _ => unreachable!("bit width not supported"),
    }
}

/// Encode the arguments of a function
pub fn encode_arguments(args: &[BorshToken]) -> Vec<u8> {
    let mut encoded: Vec<u8> = Vec::new();
    for item in args {
        item.encode(&mut encoded);
    }

    encoded
}

/// Decode a parameter at a given offset
pub fn decode_at_offset(
    data: &[u8],
    offset: &mut usize,
    ty: &IdlType,
    custom_types: &[IdlTypeDefinition],
) -> BorshToken {
    match ty {
        IdlType::PublicKey => {
            let read = &data[*offset..(*offset + 32)];
            (*offset) += 32;
            BorshToken::Address(<[u8; 32]>::try_from(read).unwrap())
        }

        IdlType::U8
        | IdlType::U16
        | IdlType::U32
        | IdlType::U64
        | IdlType::U128
        | IdlType::U256 => {
            let decoding_width = integer_byte_width(ty);
            let bigint =
                BigInt::from_bytes_le(Sign::Plus, &data[*offset..(*offset + decoding_width)]);
            (*offset) += decoding_width;
            BorshToken::Uint {
                width: (decoding_width * 8) as u16,
                value: bigint,
            }
        }

        IdlType::I8
        | IdlType::I16
        | IdlType::I32
        | IdlType::I64
        | IdlType::I128
        | IdlType::I256 => {
            let decoding_width = integer_byte_width(ty);
            let bigint = BigInt::from_signed_bytes_le(&data[*offset..(*offset + decoding_width)]);
            (*offset) += decoding_width;
            BorshToken::Int {
                width: (decoding_width * 8) as u16,
                value: bigint,
            }
        }

        IdlType::Bool => {
            let val = data[*offset] == 1;
            (*offset) += 1;
            BorshToken::Bool(val)
        }
        IdlType::String => {
            let mut int_data: [u8; 4] = Default::default();
            int_data.copy_from_slice(&data[*offset..(*offset + 4)]);
            let len = u32::from_le_bytes(int_data) as usize;
            (*offset) += 4;
            let read_string = std::str::from_utf8(&data[*offset..(*offset + len)]).unwrap();
            (*offset) += len;
            BorshToken::String(read_string.to_string())
        }
        IdlType::Array(ty, len) => {
            let mut read_items: Vec<BorshToken> = Vec::with_capacity(*len);
            for _ in 0..*len {
                read_items.push(decode_at_offset(data, offset, ty, custom_types));
            }
            BorshToken::FixedArray(read_items)
        }
        IdlType::Vec(ty) => {
            let mut int_data: [u8; 4] = Default::default();
            int_data.copy_from_slice(&data[*offset..(*offset + 4)]);
            let len = u32::from_le_bytes(int_data);
            (*offset) += 4;
            let mut read_items: Vec<BorshToken> = Vec::with_capacity(len as usize);
            for _ in 0..len {
                read_items.push(decode_at_offset(data, offset, ty, custom_types));
            }
            BorshToken::Array(read_items)
        }
        IdlType::Defined(value) => {
            let current_ty = custom_types
                .iter()
                .find(|item| &item.name == value)
                .unwrap();

            match &current_ty.ty {
                IdlTypeDefinitionTy::Enum { .. } => {
                    let value = data[*offset];
                    (*offset) += 1;
                    BorshToken::Uint {
                        width: 8,
                        value: BigInt::from(value),
                    }
                }
                IdlTypeDefinitionTy::Struct { fields } => {
                    let mut read_items: Vec<BorshToken> = Vec::with_capacity(fields.len());
                    for item in fields {
                        read_items.push(decode_at_offset(data, offset, &item.ty, custom_types));
                    }

                    BorshToken::Tuple(read_items)
                }
                IdlTypeDefinitionTy::Alias { value } => {
                    decode_at_offset(data, offset, value, custom_types)
                }
            }
        }
        IdlType::Bytes => {
            let mut int_data: [u8; 4] = Default::default();
            int_data.copy_from_slice(&data[*offset..(*offset + 4)]);
            let len = u32::from_le_bytes(int_data) as usize;
            (*offset) += 4;
            let read_data = &data[*offset..(*offset + len)];
            (*offset) += len;
            BorshToken::Bytes(read_data.to_vec())
        }

        IdlType::Option(_)
        | IdlType::F32
        | IdlType::F64
        | IdlType::Generic(..)
        | IdlType::DefinedWithTypeArgs { .. }
        | IdlType::GenericLenArray(..) => {
            unreachable!("Type not available in Solidity")
        }
    }
}

fn integer_byte_width(ty: &IdlType) -> usize {
    match ty {
        IdlType::U8 | IdlType::I8 => 1,
        IdlType::U16 | IdlType::I16 => 2,
        IdlType::U32 | IdlType::I32 => 4,
        IdlType::U64 | IdlType::I64 => 8,
        IdlType::U128 | IdlType::I128 => 16,
        IdlType::U256 | IdlType::I256 => 32,
        _ => unreachable!("Not an integer"),
    }
}

pub trait VisitorMut {
    fn visit_address(&mut self, _a: &mut [u8; 32]) {}
    fn visit_int(&mut self, _width: &mut u16, _value: &mut BigInt) {}
    fn visit_uint(&mut self, _width: &mut u16, _value: &mut BigInt) {}
    fn visit_fixed_bytes(&mut self, _v: &mut Vec<u8>) {}
    fn visit_bytes(&mut self, _v: &mut Vec<u8>) {}
    fn visit_bool(&mut self, _b: &mut bool) {}
    fn visit_string(&mut self, _s: &mut String) {}
    fn visit_fixed_array(&mut self, v: &mut Vec<BorshToken>) {
        visit_fixed_array(self, v)
    }
    fn visit_array(&mut self, v: &mut Vec<BorshToken>) {
        visit_array(self, v)
    }
    fn visit_tuple(&mut self, v: &mut Vec<BorshToken>) {
        visit_tuple(self, v)
    }
}

pub fn visit_mut<T: VisitorMut + ?Sized>(visitor: &mut T, token: &mut BorshToken) {
    match token {
        BorshToken::Address(a) => visitor.visit_address(a),
        BorshToken::Int { width, value } => visitor.visit_int(width, value),
        BorshToken::Uint { width, value } => visitor.visit_uint(width, value),
        BorshToken::FixedBytes(v) => visitor.visit_fixed_bytes(v),
        BorshToken::Bytes(v) => visitor.visit_bytes(v),
        BorshToken::Bool(b) => visitor.visit_bool(b),
        BorshToken::String(s) => visitor.visit_string(s),
        BorshToken::FixedArray(v) => visitor.visit_fixed_array(v),
        BorshToken::Array(v) => visitor.visit_array(v),
        BorshToken::Tuple(v) => visitor.visit_tuple(v),
    }
}

#[allow(clippy::ptr_arg)]
pub fn visit_fixed_array<T: VisitorMut + ?Sized>(visitor: &mut T, v: &mut Vec<BorshToken>) {
    for token in v.iter_mut() {
        visit_mut(visitor, token);
    }
}

#[allow(clippy::ptr_arg)]
pub fn visit_array<T: VisitorMut + ?Sized>(visitor: &mut T, v: &mut Vec<BorshToken>) {
    for token in v.iter_mut() {
        visit_mut(visitor, token);
    }
}

#[allow(clippy::ptr_arg)]
pub fn visit_tuple<T: VisitorMut + ?Sized>(visitor: &mut T, v: &mut Vec<BorshToken>) {
    for token in v.iter_mut() {
        visit_mut(visitor, token);
    }
}
