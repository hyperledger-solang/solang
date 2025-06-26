#![warn(clippy::pedantic)]

use crate::tests;

#[test]
fn api() {
    tests(&[
        (&["abi"], &["afterCorrupt", "n", "withinArray"]),
        (&["emit"], &["E", "library", "n", "renounceOwnership"]),
        (
            &[r"msg\.sender"],
            &[
                r"address => function\(\) internal",
                "library",
                "renounceOwnership",
            ],
        ),
        (&[r"tx\.origin"], &[]),
    ]);
}
