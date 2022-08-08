// SPDX-License-Identifier: Apache-2.0

use tiny_keccak::{Hasher, Keccak};

/// Returns an address in https://github.com/ethereum/EIPs/blob/master/EIPS/eip-55.md format
/// Assumes the src is hex number, starting with 0x, no underscores and 40 hexdigits long,
/// i.e. an ethereum address.
pub fn to_hexstr_eip55(src: &str) -> String {
    assert!(src.starts_with("0x"));
    assert!(src.chars().skip(2).all(|c| c.is_ascii_hexdigit()));

    let address: String = src
        .chars()
        .skip(2)
        .map(|c| c.to_ascii_lowercase())
        .collect();

    let mut hasher = Keccak::v256();
    hasher.update(address.as_bytes());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    "0x".chars()
        .chain(address.chars().enumerate().map(|(i, c)| {
            match c {
                '0'..='9' => c,
                'a'..='f' => {
                    // hash is 32 bytes; find the i'th "nibble"
                    let nibble = hash[i >> 1] >> if (i & 1) != 0 { 0 } else { 4 };

                    if (nibble & 8) != 0 {
                        c.to_ascii_uppercase()
                    } else {
                        c
                    }
                }
                _ => unreachable!(),
            }
        }))
        .collect()
}

#[test]
fn test_is_hexstr_eip55() {
    fn is_hexstr_eip55(s: &str) -> bool {
        to_hexstr_eip55(s) == s
    }

    assert!(is_hexstr_eip55(
        "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"
    ));
    assert!(is_hexstr_eip55(
        "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359"
    ));
    assert!(is_hexstr_eip55(
        "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB"
    ));
    assert!(is_hexstr_eip55(
        "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb"
    ));
}
