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
        r##"
        contract foo {
            function test() public {
                bytes s = "ABCD";

                assert(s.length == 4);

                s[0] = 0x41;
                s[1] = 0x42;
                s[2] = 0x43;
                s[3] = 0x44;
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn string_compare() {
    // compare literal to literal. This should be compile-time thing
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public {
                assert(hex"414243" == 'ABC');

                assert(hex'414243' != "ABD");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn string_concat() {
    // concat literal and literal. This should be compile-time thing
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public {
                assert(hex"41424344" == "AB" + "CD");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public {
                string s1 = "x";
                string s2 = "asdfasdf";

                assert(s1 + " foo" == "x foo");
                assert("bar " + s1 == "bar x");

                assert(s1 + s2 == "xasdfasdf");
            }
        }"##,
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
        r##"
        contract foo {
            function test() public returns (string) {
                return "foobar";
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    assert_eq!(runtime.vm.output, Val("foobar".to_string()).encode());

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public returns (int8[4], string, bool) {
                return ([ int8(120), 3, -127, 64], "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.",
                true);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    assert_eq!(runtime.vm.output, Ret3([ 120, 3, -127, 64], "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.".to_string(), true).encode());

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.function("test", Vec::new());

    assert_eq!(runtime.vm.output, Ret3([ 120, 3, -127, 64], "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.".to_string(), true).encode());

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test() public returns (string[]) {
                string[] x = new string[](3);

                x[0] = "abc";
                x[1] = "dl";
                x[2] = "asdf";

                return x;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.vm.output,
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
        r##"contract foo {
            function test() public {
                string dec = abi.decode(hex"0c414141", (string));
                //assert(dec == "AAA");
            }
        }"##,
    );
    runtime.function("test", vec![]);

    // we should try lengths: 0 to 63, 64 to 0x800
    let mut runtime = build_solidity(
        r##"
        contract foo {
            function test(string s) public returns (string){
                return " " + s + " ";
            }
        }"##,
    );

    let moby_dick_first_para = "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.";

    runtime.function("test", Val("foobar".to_string()).encode());
    assert_eq!(runtime.vm.output, Val(" foobar ".to_string()).encode());

    runtime.function("test", Val(moby_dick_first_para.to_string()).encode());

    assert_eq!(
        runtime.vm.output,
        Val(format!(" {moby_dick_first_para} ")).encode()
    );

    let mut rng = rand::thread_rng();

    for len in 0x4000 - 10..0x4000 + 10 {
        let mut s = Vec::new();

        s.resize(len, 0);

        rng.fill(&mut s[..]);

        let mut runtime = build_solidity(
            r##"
            contract foo {
                function test(bytes s) public returns (bytes){
                    return hex"fe" + s;
                }
            }"##,
        );

        let arg = ValB(s.clone()).encode();

        runtime.function("test", arg.clone());

        s.insert(0, 0xfeu8);

        let ret = ValB(s).encode();

        assert_eq!(runtime.vm.output, ret);
    }
}

#[test]
fn string_storage() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(String);

    let mut runtime = build_solidity(
        r##"
        contract foo {
            string bar;

            function set_bar() public {
                bar = "foobar";
            }

            function get_bar() public returns (string) {
                return bar;
            }

        }"##,
    );

    runtime.function("set_bar", Vec::new());

    assert_eq!(
        runtime.store.get(&(runtime.vm.account, [0u8; 32])).unwrap(),
        b"foobar"
    );

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.vm.output, Val("foobar".to_string()).encode());
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
        r##"
        contract foo {
            bytes bar = hex"aabbccddeeff";

            function get_index(uint32 index) public returns (bytes1) {
                return bar[index];
            }

            function get_index64(uint64 index) public returns (bytes1) {
                return bar[index];
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("get_index", Arg(1).encode());

    assert_eq!(runtime.vm.output, Ret(0xbb).encode());

    for i in 0..6 {
        runtime.function("get_index64", Arg64(i).encode());

        let vals = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];

        assert_eq!(runtime.vm.output, [Ret(vals[i as usize])].encode());
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

    assert_eq!(runtime.vm.output, vec!(0u8).encode());

    runtime.function("push", 0xe8u8.encode());

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.vm.output, vec!(0u8, 0xe8u8).encode());

    runtime.function("pop", Vec::new());

    assert_eq!(runtime.vm.output, 0xe8u8.encode());

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.vm.output, vec!(0u8).encode());
}

#[test]
fn bytes_storage_subscript() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Arg(u32, u8);

    let mut runtime = build_solidity(
        r##"
        contract foo {
            bytes bar = hex"aabbccddeeff";

            function set_index(uint32 index, bytes1 val) public {
                bar[index] = val;
            }

            function get_bar() public returns (bytes) {
                return bar;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("set_index", Arg(1, 0x33).encode());

    assert_eq!(
        runtime.store.get(&(runtime.vm.account, [0u8; 32])).unwrap(),
        &vec!(0xaa, 0x33, 0xcc, 0xdd, 0xee, 0xff)
    );

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("or", Arg(1, 0x50).encode());

    assert_eq!(
        runtime.store.get(&(runtime.vm.account, [0u8; 32])).unwrap(),
        &vec!(0xde, 0xfd, 0xca, 0xfe)
    );

    runtime.function("and", Arg(3, 0x7f).encode());

    assert_eq!(
        runtime.store.get(&(runtime.vm.account, [0u8; 32])).unwrap(),
        &vec!(0xde, 0xfd, 0xca, 0x7e)
    );

    runtime.function("xor", Arg(2, 0xff).encode());

    assert_eq!(
        runtime.store.get(&(runtime.vm.account, [0u8; 32])).unwrap(),
        &vec!(0xde, 0xfd, 0x35, 0x7e)
    );
}

#[test]
fn bytes_memory_subscript() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Arg(u32, u8);

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(Vec<u8>);

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function set_index(uint32 index, bytes1 val) public returns (bytes) {
                bytes bar = hex"aabbccddeeff";

                bar[index] = val;

                return bar;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("set_index", Arg(1, 0x33).encode());

    assert_eq!(
        runtime.vm.output,
        Ret(vec!(0xaa, 0x33, 0xcc, 0xdd, 0xee, 0xff)).encode()
    );

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("or", Arg(1, 0x50).encode());

    assert_eq!(
        runtime.vm.output,
        Ret(vec!(0xde, 0xfd, 0xca, 0xfe)).encode()
    );

    runtime.function("and", Arg(3, 0x7f).encode());

    assert_eq!(
        runtime.vm.output,
        Ret(vec!(0xde, 0xad, 0xca, 0x7e)).encode()
    );

    runtime.function("xor", Arg(2, 0xff).encode());

    assert_eq!(
        runtime.vm.output,
        Ret(vec!(0xde, 0xad, 0x35, 0xfe)).encode()
    );
}

#[test]
fn string_escape() {
    let mut runtime = build_solidity(
        r##"
        contract カラス {
            function カラス$() public {
                print(" \u20ac \x41 \f\b\r\n\v\\\'\"\t");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("カラス$", Vec::new());

    assert_eq!(runtime.printbuf, " € A \u{c}\u{8}\r\n\u{b}\\'\"\t");
}
