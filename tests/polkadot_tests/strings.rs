// SPDX-License-Identifier: Apache-2.0

use parity_scale_codec::{Decode, Encode};
use rand::Rng;

use crate::build_solidity;

#[test]
fn more_tests() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public {
                string s = new string(10);

                assert(s.length == 10);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public {
                bytes s = new bytes(2);

                s[0] = 0x41;
                s[1] = 0x42;

                assert(s.length == 2);

                assert(s[0] == 0x41);
                assert(s[1] == 0x42);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function ref_test(bytes n) private {
                n[1] = 102;

                n = new bytes(10);
                // new reference
                n[1] = 104;
            }

            function test() public {
                bytes s = new bytes(2);

                s[0] = 0x41;
                s[1] = 0x42;

                assert(s.length == 2);

                ref_test(s);

                assert(s[0] == 0x41);
                assert(s[1] == 102);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                bytes s = "ABCD";

                assert(s.length == 4);

                s[0] = 0x41;
                s[1] = 0x42;
                s[2] = 0x43;
                s[3] = 0x44;
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn string_compare() {
    // compare literal to literal. This should be compile-time thing
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                assert(hex"414243" == 'ABC');

                assert(hex'414243' != "ABD");
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function lets_compare1(string s) private returns (bool) {
                return s == unicode'the quick brown fox jumps over the lazy dog';
            }

            function lets_compare2(string s) private returns (bool) {
                return unicode"the quick brown fox jumps over the lazy dog" == s;
            }

            function test() public {
                string s1 = "the quick brown fox jumps over the lazy dog";

                assert(lets_compare1(s1));
                assert(lets_compare2(s1));

                string s2 = "the quick brown dog jumps over the lazy fox";

                assert(!lets_compare1(s2));
                assert(!lets_compare2(s2));

                assert(s1 != s2);

                s1 = "the quick brown dog jumps over the lazy fox";

                assert(s1 == s2);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn string_concat() {
    // concat literal and literal. This should be compile-time thing
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                assert(hex"41424344" == bytes.concat("AB", "CD", bytes(string.concat())));

                bytes2 AB = 0x4142;
                bytes2 CD = 0x4344;

                assert(bytes.concat(AB, CD) == hex"41424344");
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                string s1 = "x";
                string s2 = "asdfasdf";

                assert(string.concat(s1, " foo") == "x foo");
                assert(string.concat("bar ", s1) == "bar x");

                assert(string.concat(s1, s2) == "xasdfasdf");
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn string_abi_encode() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(String);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret3([i8; 4], String, bool);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct RetStringArray(Vec<String>);

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public returns (string) {
                return "foobar";
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    assert_eq!(runtime.output(), Val("foobar".to_string()).encode());

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public returns (int8[4], string, bool) {
                return ([ int8(120), 3, -127, 64], "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.",
                true);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    assert_eq!(runtime.output(), Ret3([ 120, 3, -127, 64], "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.".to_string(), true).encode());

    let mut runtime = build_solidity(
        r#"
        struct s {
            int8[4] f1;
            string f2;
            bool f3;
        }

        contract foo {
            function test() public returns (s) {
                return s({ f1: [ int8(120), 3, -127, 64], f2: "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.",
                f3: true});
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    assert_eq!(runtime.output(), Ret3([ 120, 3, -127, 64], "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.".to_string(), true).encode());

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public returns (string[]) {
                string[] x = new string[](3);

                x[0] = "abc";
                x[1] = "dl";
                x[2] = "asdf";

                return x;
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.output(),
        RetStringArray(vec!(
            "abc".to_string(),
            "dl".to_string(),
            "asdf".to_string()
        ))
        .encode()
    );
}

#[test]
fn string_abi_decode() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(String);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct ValB(Vec<u8>);

    let mut runtime = build_solidity(
        r#"contract foo {
            function test() public {
                string dec = abi.decode(hex"0c414141", (string));
                assert(dec == "AAA");
            }
        }"#,
    );
    runtime.function("test", vec![]);

    // we should try lengths: 0 to 63, 64 to 0x800
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test(string s) public returns (string){
                return string.concat(" ", s, " ");
            }
        }"#,
    );

    let moby_dick_first_para = "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.";

    runtime.function("test", Val("foobar".to_string()).encode());
    assert_eq!(runtime.output(), Val(" foobar ".to_string()).encode());

    runtime.function("test", Val(moby_dick_first_para.to_string()).encode());

    assert_eq!(
        runtime.output(),
        Val(format!(" {moby_dick_first_para} ")).encode()
    );

    let mut rng = rand::thread_rng();

    for len in 0x4000 - 10..0x4000 + 10 {
        let mut s = vec![0; len];
        rng.fill(&mut s[..]);

        let mut runtime = build_solidity(
            r#"
            contract foo {
                function test(bytes s) public returns (bytes){
                    return bytes.concat(hex"fe", s);
                }
            }"#,
        );

        let arg = ValB(s.clone()).encode();

        runtime.function("test", arg.clone());

        s.insert(0, 0xfeu8);

        let ret = ValB(s).encode();

        assert_eq!(runtime.output(), ret);
    }
}

#[test]
fn string_storage() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(String);

    let mut runtime = build_solidity(
        r#"
        contract foo {
            string bar;

            function set_bar() public {
                bar = "foobar";
            }

            function get_bar() public returns (string) {
                return bar;
            }

        }"#,
    );

    runtime.function("set_bar", Vec::new());

    assert_eq!(runtime.storage()[&[0; 32]], b"foobar");

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.output(), Val("foobar".to_string()).encode());
}

#[test]
fn bytes_storage() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(Vec<u8>);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(u8);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Arg(u32);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Arg64(u64);

    let mut runtime = build_solidity(
        r#"
        contract foo {
            bytes bar = hex"aabbccddeeff";

            function get_index(uint32 index) public returns (bytes1) {
                return bar[index];
            }

            function get_index64(uint64 index) public returns (bytes1) {
                return bar[index];
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("get_index", Arg(1).encode());

    assert_eq!(runtime.output(), Ret(0xbb).encode());

    for i in 0..6 {
        runtime.function("get_index64", Arg64(i).encode());

        let vals = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];

        assert_eq!(runtime.output(), [Ret(vals[i as usize])].encode());
    }

    let mut runtime = build_solidity(
        r##"
        contract foo {
            bytes bar;

            function push_test() public {
                bytes1 x = bar.push();
                assert(bar.length == 1);
            }

            function push(byte x) public {
                bar.push(x);
            }

            function pop() public returns (byte) {
                return bar.pop();
            }

            function get_bar() public returns (bytes) {
                return bar;
            }
        }"##,
    );

    runtime.function("push_test", Vec::new());

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.output(), vec!(0u8).encode());

    runtime.function("push", 0xe8u8.encode());

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.output(), vec!(0u8, 0xe8u8).encode());

    runtime.function("pop", Vec::new());

    assert_eq!(runtime.output(), 0xe8u8.encode());

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.output(), vec!(0u8).encode());
}

#[test]
fn bytes_storage_subscript() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Arg(u32, u8);

    let mut runtime = build_solidity(
        r#"
        contract foo {
            bytes bar = hex"aabbccddeeff";

            function set_index(uint32 index, bytes1 val) public {
                bar[index] = val;
            }

            function get_bar() public returns (bytes) {
                return bar;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("set_index", Arg(1, 0x33).encode());

    assert_eq!(
        runtime.storage()[&[0; 32]],
        vec!(0xaa, 0x33, 0xcc, 0xdd, 0xee, 0xff)
    );

    let mut runtime = build_solidity(
        r#"
        contract foo {
            bytes bar = hex"deadcafe";

            function or(uint32 index, bytes1 val) public {
                bar[index] |= val;
            }

            function xor(uint32 index, bytes1 val) public {
                bar[index] ^= val;
            }

            function and(uint32 index, bytes1 val) public {
                bar[index] &= val;
            }

            function get() public returns (bytes) {
                return bar;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("or", Arg(1, 0x50).encode());

    assert_eq!(runtime.storage()[&[0; 32]], vec!(0xde, 0xfd, 0xca, 0xfe));

    runtime.function("and", Arg(3, 0x7f).encode());

    assert_eq!(runtime.storage()[&[0; 32]], vec!(0xde, 0xfd, 0xca, 0x7e));

    runtime.function("xor", Arg(2, 0xff).encode());

    assert_eq!(runtime.storage()[&[0; 32]], vec!(0xde, 0xfd, 0x35, 0x7e));
}

#[test]
fn bytes_memory_subscript() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Arg(u32, u8);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(Vec<u8>);

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function set_index(uint32 index, bytes1 val) public returns (bytes) {
                bytes bar = hex"aabbccddeeff";

                bar[index] = val;

                return bar;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("set_index", Arg(1, 0x33).encode());

    assert_eq!(
        runtime.output(),
        Ret(vec!(0xaa, 0x33, 0xcc, 0xdd, 0xee, 0xff)).encode()
    );

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function or(uint32 index, bytes1 val) public returns (bytes) {
                bytes bar = hex"deadcafe";

                bar[index] |= val;

                return bar;
            }

            function xor(uint32 index, bytes1 val) public returns (bytes) {
                bytes bar = hex"deadcafe";

                bar[index] ^= val;

                return bar;
            }

            function and(uint32 index, bytes1 val) public returns (bytes) {
                bytes bar = hex"deadcafe";

                bar[index] &= val;

                return bar;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("or", Arg(1, 0x50).encode());

    assert_eq!(runtime.output(), Ret(vec!(0xde, 0xfd, 0xca, 0xfe)).encode());

    runtime.function("and", Arg(3, 0x7f).encode());

    assert_eq!(runtime.output(), Ret(vec!(0xde, 0xad, 0xca, 0x7e)).encode());

    runtime.function("xor", Arg(2, 0xff).encode());

    assert_eq!(runtime.output(), Ret(vec!(0xde, 0xad, 0x35, 0xfe)).encode());
}

#[test]
fn string_escape() {
    let mut runtime = build_solidity(
        r#"
        contract カラス {
            function カラス$() public {
                print(" \u20ac \x41 \f\b\r\n\v\\\'\"\t");
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("カラス$", Vec::new());

    assert_eq!(
        runtime.debug_buffer(),
        "print:  € A \u{c}\u{8}\r\n\u{b}\\'\"\t,\n"
    );
}

#[test]
fn long_string() {
    // String used here (containing some UTF-8 chars, which is why bytes length != string length):
    // "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me."
    let mut runtime = build_solidity(
        r#"
        contract LongString {
            function decode() pure public {
                bytes enc = hex"6d1143616c6c206d65204973686d61656c2e20536f6d652079656172732061676fe280946e65766572206d696e6420686f77206c6f6e6720707265636973656c79e28094686176696e67206c6974746c65206f72206e6f206d6f6e657920696e206d792070757273652c20616e64206e6f7468696e6720706172746963756c617220746f20696e746572657374206d65206f6e2073686f72652c20492074686f75676874204920776f756c64207361696c2061626f75742061206c6974746c6520616e642073656520746865207761746572792070617274206f662074686520776f726c642e20497420697320612077617920492068617665206f662064726976696e67206f6666207468652073706c65656e20616e6420726567756c6174696e67207468652063697263756c6174696f6e2e205768656e6576657220492066696e64206d7973656c662067726f77696e67206772696d2061626f757420746865206d6f7574683b207768656e6576657220697420697320612064616d702c206472697a7a6c79204e6f76656d62657220696e206d7920736f756c3b207768656e6576657220492066696e64206d7973656c6620696e766f6c756e746172696c792070617573696e67206265666f726520636f6666696e2077617265686f757365732c20616e64206272696e67696e67207570207468652072656172206f662065766572792066756e6572616c2049206d6565743b20616e6420657370656369616c6c79207768656e65766572206d79206879706f7320676574207375636820616e2075707065722068616e64206f66206d652c20746861742069742072657175697265732061207374726f6e67206d6f72616c207072696e6369706c6520746f2070726576656e74206d652066726f6d2064656c696265726174656c79207374657070696e6720696e746f20746865207374726565742c20616e64206d6574686f646963616c6c79206b6e6f636b696e672070656f706c65e28099732068617473206f6666e280947468656e2c2049206163636f756e7420697420686967682074696d6520746f2067657420746f2073656120617320736f6f6e20617320492063616e2e2054686973206973206d79207375627374697475746520666f7220706973746f6c20616e642062616c6c2e20576974682061207068696c6f736f70686963616c20666c6f7572697368204361746f207468726f77732068696d73656c662075706f6e206869732073776f72643b20492071756965746c792074616b6520746f2074686520736869702e205468657265206973206e6f7468696e672073757270726973696e6720696e20746869732e204966207468657920627574206b6e65772069742c20616c6d6f737420616c6c206d656e20696e207468656972206465677265652c20736f6d652074696d65206f72206f746865722c20636865726973682076657279206e6561726c79207468652073616d65206665656c696e677320746f776172647320746865206f6365616e2077697468206d652e";
                string dec = abi.decode(enc, (string));
                assert(dec.length == 1115);
                assert(dec == "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.");

                bytes enc2 = hex"b14543616c6c206d65204973686d61656c2e20536f6d652079656172732061676fe280946e65766572206d696e6420686f77206c6f6e6720707265636973656c79e28094686176696e67206c6974746c65206f72206e6f206d6f6e657920696e206d792070757273652c20616e64206e6f7468696e6720706172746963756c617220746f20696e746572657374206d65206f6e2073686f72652c20492074686f75676874204920776f756c64207361696c2061626f75742061206c6974746c6520616e642073656520746865207761746572792070617274206f662074686520776f726c642e20497420697320612077617920492068617665206f662064726976696e67206f6666207468652073706c65656e20616e6420726567756c6174696e67207468652063697263756c6174696f6e2e205768656e6576657220492066696e64206d7973656c662067726f77696e67206772696d2061626f757420746865206d6f7574683b207768656e6576657220697420697320612064616d702c206472697a7a6c79204e6f76656d62657220696e206d7920736f756c3b207768656e6576657220492066696e64206d7973656c6620696e766f6c756e746172696c792070617573696e67206265666f726520636f6666696e2077617265686f757365732c20616e64206272696e67696e67207570207468652072656172206f662065766572792066756e6572616c2049206d6565743b20616e6420657370656369616c6c79207768656e65766572206d79206879706f7320676574207375636820616e2075707065722068616e64206f66206d652c20746861742069742072657175697265732061207374726f6e67206d6f72616c207072696e6369706c6520746f2070726576656e74206d652066726f6d2064656c696265726174656c79207374657070696e6720696e746f20746865207374726565742c20616e64206d6574686f646963616c6c79206b6e6f636b696e672070656f706c65e28099732068617473206f6666e280947468656e2c2049206163636f756e7420697420686967682074696d6520746f2067657420746f2073656120617320736f6f6e20617320492063616e2e2054686973206973206d79207375627374697475746520666f7220706973746f6c20616e642062616c6c2e20576974682061207068696c6f736f70686963616c20666c6f7572697368204361746f207468726f77732068696d73656c662075706f6e206869732073776f72643b20492071756965746c792074616b6520746f2074686520736869702e205468657265206973206e6f7468696e672073757270726973696e6720696e20746869732e204966207468657920627574206b6e65772069742c20616c6d6f737420616c6c206d656e20696e207468656972206465677265652c20736f6d652074696d65206f72206f746865722c20636865726973682076657279206e6561726c79207468652073616d65206665656c696e677320746f776172647320746865206f6365616e2077697468206d652e43616c6c206d65204973686d61656c2e20536f6d652079656172732061676fe280946e65766572206d696e6420686f77206c6f6e6720707265636973656c79e28094686176696e67206c6974746c65206f72206e6f206d6f6e657920696e206d792070757273652c20616e64206e6f7468696e6720706172746963756c617220746f20696e746572657374206d65206f6e2073686f72652c20492074686f75676874204920776f756c64207361696c2061626f75742061206c6974746c6520616e642073656520746865207761746572792070617274206f662074686520776f726c642e20497420697320612077617920492068617665206f662064726976696e67206f6666207468652073706c65656e20616e6420726567756c6174696e67207468652063697263756c6174696f6e2e205768656e6576657220492066696e64206d7973656c662067726f77696e67206772696d2061626f757420746865206d6f7574683b207768656e6576657220697420697320612064616d702c206472697a7a6c79204e6f76656d62657220696e206d7920736f756c3b207768656e6576657220492066696e64206d7973656c6620696e766f6c756e746172696c792070617573696e67206265666f726520636f6666696e2077617265686f757365732c20616e64206272696e67696e67207570207468652072656172206f662065766572792066756e6572616c2049206d6565743b20616e6420657370656369616c6c79207768656e65766572206d79206879706f7320676574207375636820616e2075707065722068616e64206f66206d652c20746861742069742072657175697265732061207374726f6e67206d6f72616c207072696e6369706c6520746f2070726576656e74206d652066726f6d2064656c696265726174656c79207374657070696e6720696e746f20746865207374726565742c20616e64206d6574686f646963616c6c79206b6e6f636b696e672070656f706c65e28099732068617473206f6666e280947468656e2c2049206163636f756e7420697420686967682074696d6520746f2067657420746f2073656120617320736f6f6e20617320492063616e2e2054686973206973206d79207375627374697475746520666f7220706973746f6c20616e642062616c6c2e20576974682061207068696c6f736f70686963616c20666c6f7572697368204361746f207468726f77732068696d73656c662075706f6e206869732073776f72643b20492071756965746c792074616b6520746f2074686520736869702e205468657265206973206e6f7468696e672073757270726973696e6720696e20746869732e204966207468657920627574206b6e65772069742c20616c6d6f737420616c6c206d656e20696e207468656972206465677265652c20736f6d652074696d65206f72206f746865722c20636865726973682076657279206e6561726c79207468652073616d65206665656c696e677320746f776172647320746865206f6365616e2077697468206d652e43616c6c206d65204973686d61656c2e20536f6d652079656172732061676fe280946e65766572206d696e6420686f77206c6f6e6720707265636973656c79e28094686176696e67206c6974746c65206f72206e6f206d6f6e657920696e206d792070757273652c20616e64206e6f7468696e6720706172746963756c617220746f20696e746572657374206d65206f6e2073686f72652c20492074686f75676874204920776f756c64207361696c2061626f75742061206c6974746c6520616e642073656520746865207761746572792070617274206f662074686520776f726c642e20497420697320612077617920492068617665206f662064726976696e67206f6666207468652073706c65656e20616e6420726567756c6174696e67207468652063697263756c6174696f6e2e205768656e6576657220492066696e64206d7973656c662067726f77696e67206772696d2061626f757420746865206d6f7574683b207768656e6576657220697420697320612064616d702c206472697a7a6c79204e6f76656d62657220696e206d7920736f756c3b207768656e6576657220492066696e64206d7973656c6620696e766f6c756e746172696c792070617573696e67206265666f726520636f6666696e2077617265686f757365732c20616e64206272696e67696e67207570207468652072656172206f662065766572792066756e6572616c2049206d6565743b20616e6420657370656369616c6c79207768656e65766572206d79206879706f7320676574207375636820616e2075707065722068616e64206f66206d652c20746861742069742072657175697265732061207374726f6e67206d6f72616c207072696e6369706c6520746f2070726576656e74206d652066726f6d2064656c696265726174656c79207374657070696e6720696e746f20746865207374726565742c20616e64206d6574686f646963616c6c79206b6e6f636b696e672070656f706c65e28099732068617473206f6666e280947468656e2c2049206163636f756e7420697420686967682074696d6520746f2067657420746f2073656120617320736f6f6e20617320492063616e2e2054686973206973206d79207375627374697475746520666f7220706973746f6c20616e642062616c6c2e20576974682061207068696c6f736f70686963616c20666c6f7572697368204361746f207468726f77732068696d73656c662075706f6e206869732073776f72643b20492071756965746c792074616b6520746f2074686520736869702e205468657265206973206e6f7468696e672073757270726973696e6720696e20746869732e204966207468657920627574206b6e65772069742c20616c6d6f737420616c6c206d656e20696e207468656972206465677265652c20736f6d652074696d65206f72206f746865722c20636865726973682076657279206e6561726c79207468652073616d65206665656c696e677320746f776172647320746865206f6365616e2077697468206d652e43616c6c206d65204973686d61656c2e20536f6d652079656172732061676fe280946e65766572206d696e6420686f77206c6f6e6720707265636973656c79e28094686176696e67206c6974746c65206f72206e6f206d6f6e657920696e206d792070757273652c20616e64206e6f7468696e6720706172746963756c617220746f20696e746572657374206d65206f6e2073686f72652c20492074686f75676874204920776f756c64207361696c2061626f75742061206c6974746c6520616e642073656520746865207761746572792070617274206f662074686520776f726c642e20497420697320612077617920492068617665206f662064726976696e67206f6666207468652073706c65656e20616e6420726567756c6174696e67207468652063697263756c6174696f6e2e205768656e6576657220492066696e64206d7973656c662067726f77696e67206772696d2061626f757420746865206d6f7574683b207768656e6576657220697420697320612064616d702c206472697a7a6c79204e6f76656d62657220696e206d7920736f756c3b207768656e6576657220492066696e64206d7973656c6620696e766f6c756e746172696c792070617573696e67206265666f726520636f6666696e2077617265686f757365732c20616e64206272696e67696e67207570207468652072656172206f662065766572792066756e6572616c2049206d6565743b20616e6420657370656369616c6c79207768656e65766572206d79206879706f7320676574207375636820616e2075707065722068616e64206f66206d652c20746861742069742072657175697265732061207374726f6e67206d6f72616c207072696e6369706c6520746f2070726576656e74206d652066726f6d2064656c696265726174656c79207374657070696e6720696e746f20746865207374726565742c20616e64206d6574686f646963616c6c79206b6e6f636b696e672070656f706c65e28099732068617473206f6666e280947468656e2c2049206163636f756e7420697420686967682074696d6520746f2067657420746f2073656120617320736f6f6e20617320492063616e2e2054686973206973206d79207375627374697475746520666f7220706973746f6c20616e642062616c6c2e20576974682061207068696c6f736f70686963616c20666c6f7572697368204361746f207468726f77732068696d73656c662075706f6e206869732073776f72643b20492071756965746c792074616b6520746f2074686520736869702e205468657265206973206e6f7468696e672073757270726973696e6720696e20746869732e204966207468657920627574206b6e65772069742c20616c6d6f737420616c6c206d656e20696e207468656972206465677265652c20736f6d652074696d65206f72206f746865722c20636865726973682076657279206e6561726c79207468652073616d65206665656c696e677320746f776172647320746865206f6365616e2077697468206d652e";
                string dec2 = abi.decode(enc2, (string));
                assert(dec2.length == 1115*4);
                assert(dec2 == "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.");
            }
        }
        "#,
    );
    runtime.constructor(0, vec![]);
    runtime.function("decode", vec![]);
}

#[test]
fn string_encoded_length() {
    let mut runtime = build_solidity(
        r#"
        contract StrLen {
            function strlen() public pure {
                    // Length < 0x40 is expected to have 1 byte length width
                    bytes e63 = abi.encode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
                    assert(e63.length == 64);
                    assert(e63[0] == 0xfc);

                    // Length >= 0x40 is expected to have 2 bytes length width
                    bytes e64 = abi.encode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
                    assert(e64.length == 64 + 2);
                    assert(e64[0] == 0x01);
                    assert(e64[1] == 0x01);

                    // Length >= 0x4000 is expected to have 4 bytes length width
                    bytes e16384 = abi.encode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
                    assert(e16384.length == 16384 + 4);
                    assert(e16384[0] == 0x02);
                    assert(e16384[1] == 0x00);
                    assert(e16384[2] == 0x01);
                    assert(e16384[3] == 0x00);
            }
        }"#,
    );
    runtime.function("strlen", vec![]);
}
