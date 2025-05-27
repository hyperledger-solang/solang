#![warn(clippy::pedantic)]

use crate::test;

#[test]
fn tx_origin() {
    test(&[r"tx\.origin"], &[]);
}
