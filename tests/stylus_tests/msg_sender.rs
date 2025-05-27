#![warn(clippy::pedantic)]

use crate::test;

#[test]
fn msg_sender() {
    test(&[r"msg\.sender"], &["emit", "library"]);
}
