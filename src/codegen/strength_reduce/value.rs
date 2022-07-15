use super::Bits;
use bitvec::prelude::BitArray;
use bitvec::prelude::Lsb0;
use num_bigint::{BigInt, Sign};
use num_traits::Signed;
use num_traits::Zero;
use std::collections::HashSet;
use std::fmt;

#[derive(Eq, Hash, Debug, Clone, PartialEq)]
pub(super) struct Value {
    // which bits are known
    pub(super) known_bits: BitArray<Lsb0, [u8; 32]>,
    // value
    pub(super) value: BitArray<Lsb0, [u8; 32]>,
    // type
    pub(super) bits: usize,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.all_known() {
            write!(
                f,
                "{}",
                BigInt::from_signed_bytes_le(self.value.as_buffer())
            )
        } else if self.all_unknown() {
            write!(f, "unknown")
        } else {
            write!(
                f,
                "{} k:{}",
                BigInt::from_signed_bytes_le(self.value.as_buffer()),
                hex::encode(self.value[0..self.bits].as_slice())
            )
        }
    }
}

#[allow(unused)]
fn dump_set(name: &str, set: &HashSet<Value>) {
    println!(
        "{}:{}",
        name,
        set.iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<String>>()
            .join(",")
    );
}

/// Is the value set just a single constant
pub(super) fn is_single_constant(set: &HashSet<Value>) -> Option<BigInt> {
    if set.len() == 1 {
        let v = set.iter().next().unwrap();

        if v.all_known() {
            return Some(BigInt::from_signed_bytes_le(v.value[0..v.bits].as_slice()));
        }
    }

    None
}

/// Get the maximum unsigned value in a set
pub(super) fn set_max_signed(set: &HashSet<Value>) -> Option<BigInt> {
    let mut m = BigInt::zero();

    for v in set {
        let (sign_known, sign) = v.sign();

        if !sign_known {
            return None;
        }

        let v = if sign {
            BigInt::from_signed_bytes_le(v.get_signed_min_value().as_buffer())
        } else {
            BigInt::from_signed_bytes_le(v.get_signed_max_value().as_buffer())
        };

        if v.abs() > m.abs() {
            m = v;
        }
    }

    Some(m)
}

/// Get the maximum signed value in a set
pub(super) fn set_max_unsigned(set: &HashSet<Value>) -> BigInt {
    let mut m = BigInt::zero();

    for v in set {
        let v = BigInt::from_bytes_le(Sign::Plus, v.get_unsigned_max_value().as_buffer());

        m = std::cmp::max(v, m);
    }

    m
}

impl Value {
    /// Calculate the unsigned min value. Higher bits than the type are 0
    pub(super) fn get_unsigned_min_value(&self) -> Bits {
        self.value & self.known_bits
    }

    /// Calculate the unsigned max value. Higher bits than the type are 0
    pub(super) fn get_unsigned_max_value(&self) -> Bits {
        (BitArray::new([!0u8; 32]) & !self.known_bits) | self.value
    }

    /// Return whether the sign is known and what value it is
    pub(super) fn sign(&self) -> (bool, bool) {
        let sign_bit = self.bits - 1;

        (self.known_bits[sign_bit], self.value[sign_bit])
    }

    /// Calculate the signed max value
    pub(super) fn get_signed_max_value(&self) -> Bits {
        let negative = match self.sign() {
            (true, sign) => sign,
            (false, _) => false,
        };

        if !negative {
            // we know the value is positive; same as unsigned
            self.get_unsigned_max_value()
        } else {
            // the value might be negative. So, we want to know which bits are zero
            let mut v = self.get_unsigned_min_value();
            v[self.bits - 1..].set_all(true);
            v
        }
    }

    pub(super) fn get_signed_min_value(&self) -> Bits {
        let negative = match self.sign() {
            (true, sign) => sign,
            (false, _) => true,
        };

        if !negative {
            // we know the value is positive; same as unsigned
            self.get_unsigned_min_value()
        } else {
            // the value might be negative. So, we want to know which bits are zero
            let mut v = self.get_unsigned_max_value();
            v[self.bits - 1..].set_all(true);
            v
        }
    }

    pub(super) fn all_known(&self) -> bool {
        self.known_bits[0..self.bits].all()
    }

    pub(super) fn all_unknown(&self) -> bool {
        self.known_bits[0..self.bits].not_any()
    }

    pub(super) fn unknown(bits: usize) -> Value {
        Value {
            value: BitArray::new([0u8; 32]),
            known_bits: BitArray::new([0u8; 32]),
            bits,
        }
    }
}
