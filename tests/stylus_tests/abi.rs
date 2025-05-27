#![warn(clippy::pedantic)]

use crate::test;

#[test]
fn abi() {
    test(&["abi", "assert"], &[]);
}
