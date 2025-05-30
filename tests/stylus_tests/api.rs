#![warn(clippy::pedantic)]

use crate::tests;

#[test]
fn abi() {
    tests(&[
        (&["abi"], &["afterCorrupt", "emit", "withinArray"]),
        (
            &[r"msg\.sender"],
            &[r"address => function\(\) internal", "emit", "library"],
        ),
        (&[r"tx\.origin"], &[]),
    ]);
}
