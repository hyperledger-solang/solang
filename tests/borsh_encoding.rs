// SPDX-License-Identifier: Apache-2.0

use byte_slice_cast::AsByteSlice;
use ethabi::{Param, ParamType};
use num_bigint::{BigInt, Sign};
use num_traits::ToPrimitive;
use std::cmp::Ordering;

/// This is the token that should be used for each function call in Solana runtime tests
#[derive(Debug, PartialEq, Clone)]
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
                std::println!("len: {}", buffer.len());
                std::println!("buffer: {:?}", buffer);
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
}

/// Encode a signed integer
fn encode_int(width: u16, value: &BigInt, buffer: &mut Vec<u8>) {
    match width {
        8 => {
            let val = value.to_i8().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        16 => {
            let val = value.to_i16().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        32 => {
            let val = value.to_i32().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        64 => {
            let val = value.to_i64().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        128 => {
            let val = value.to_i128().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        n if n <= 256 => {
            let mut val = value.to_signed_bytes_le();
            let byte_width = (width / 8) as usize;
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

        _ => unimplemented!("bit width not supported"),
    }
}

/// Encode an unsigned integer
fn encode_uint(width: u16, value: &BigInt, buffer: &mut Vec<u8>) {
    match width {
        8 => {
            let val = value.to_u8().unwrap();
            buffer.push(val);
        }
        16 => {
            let val = value.to_u16().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        32 => {
            let val = value.to_u32().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        64 => {
            let val = value.to_u64().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        128 => {
            let val = value.to_u128().unwrap();
            buffer.extend_from_slice(&val.to_le_bytes());
        }
        n if n <= 256 => {
            let mut val = value.to_signed_bytes_le();
            let bytes_width = (width / 8) as usize;
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

/// Decode the output buffer of a function given the description of its parameters
pub fn decode_output(params: &[Param], data: &[u8]) -> Vec<BorshToken> {
    let mut offset: usize = 0;
    let mut decoded: Vec<BorshToken> = Vec::with_capacity(params.len());
    for item in params {
        let borsh_token = decode_at_offset(data, &mut offset, &item.kind);
        decoded.push(borsh_token);
    }

    decoded
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
fn decode_at_offset(data: &[u8], offset: &mut usize, ty: &ParamType) -> BorshToken {
    match ty {
        ParamType::Address => {
            let read = &data[*offset..(*offset + 32)];
            (*offset) += 32;
            BorshToken::Address(<[u8; 32]>::try_from(read).unwrap())
        }
        ParamType::Uint(width) => {
            let bigint = BigInt::from_bytes_le(Sign::Plus, &data[*offset..(*offset + width / 8)]);
            (*offset) += width / 8;
            BorshToken::Uint {
                width: *width as u16,
                value: bigint,
            }
        }
        ParamType::Int(width) => {
            let bigint = BigInt::from_signed_bytes_le(&data[*offset..(*offset + width / 8)]);
            (*offset) += width / 8;
            BorshToken::Int {
                width: *width as u16,
                value: bigint,
            }
        }
        ParamType::Bool => {
            let val = data[*offset] == 1;
            (*offset) += 1;
            BorshToken::Bool(val)
        }
        ParamType::String => {
            let mut int_data: [u8; 4] = Default::default();
            int_data.copy_from_slice(&data[*offset..(*offset + 4)]);
            let len = u32::from_le_bytes(int_data) as usize;
            (*offset) += 4;
            let read_string = std::str::from_utf8(&data[*offset..(*offset + len)]).unwrap();
            (*offset) += len;
            BorshToken::String(read_string.to_string())
        }
        ParamType::FixedBytes(len) => {
            let read_data = &data[*offset..(*offset + len)];
            (*offset) += len;
            BorshToken::FixedBytes(read_data.to_vec())
        }
        ParamType::FixedArray(ty, len) => {
            let mut read_items: Vec<BorshToken> = Vec::with_capacity(*len);
            for _ in 0..*len {
                read_items.push(decode_at_offset(data, offset, ty));
            }
            BorshToken::FixedArray(read_items)
        }
        ParamType::Array(ty) => {
            let mut int_data: [u8; 4] = Default::default();
            int_data.copy_from_slice(&data[*offset..(*offset + 4)]);
            let len = u32::from_le_bytes(int_data);
            (*offset) += 4;
            let mut read_items: Vec<BorshToken> = Vec::with_capacity(len as usize);
            for _ in 0..len {
                read_items.push(decode_at_offset(data, offset, ty));
            }
            BorshToken::Array(read_items)
        }
        ParamType::Tuple(items) => {
            let mut read_items: Vec<BorshToken> = Vec::with_capacity(items.len());
            for item in items {
                read_items.push(decode_at_offset(data, offset, item));
            }
            BorshToken::Tuple(read_items)
        }
        ParamType::Bytes => {
            let mut int_data: [u8; 4] = Default::default();
            int_data.copy_from_slice(&data[*offset..(*offset + 4)]);
            let len = u32::from_le_bytes(int_data) as usize;
            (*offset) += 4;
            let read_data = &data[*offset..(*offset + len)];
            (*offset) += len;
            BorshToken::Bytes(read_data.to_vec())
        }
    }
}
