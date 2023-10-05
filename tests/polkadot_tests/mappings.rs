// SPDX-License-Identifier: Apache-2.0

use parity_scale_codec::{Decode, Encode};
use rand::Rng;
use std::collections::HashMap;

use crate::build_solidity;

#[test]
fn basic() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            mapping(uint => bytes4) data;

            function test() public {
                data[1] = hex"cafedead";

                assert(data[1] == hex"cafedead");
            }
        }
        "#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract Test {
            using TestLib for TestLib.data;

            TestLib.data libdata;

            function contfunc(uint64 num) public {
                libdata.libfunc(num);
            }
        }

        library TestLib {
            using TestLib for TestLib.data;

            struct pair {
                uint64 a;
                uint64 b;
            }

            struct data {
                mapping(uint64 => pair) pairmap;
            }

            function libfunc(data storage self, uint64 value) internal {
                self.pairmap[self.pairmap[value].a].a = 1;
                self.pairmap[self.pairmap[value].b].b = 2;
            }
        }
        "##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("contfunc", 1u64.encode());
}

#[test]
fn test_uint64() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct SetArg(u64, i32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetArg(u64);

    let mut runtime = build_solidity(
        r##"
    contract foo {
        mapping(uint64 => int32) v;

        function set(uint64 index, int32 val) public {
            v[index] = val;
        }

        function get(uint64 index) public returns (int32) {
            return v[index];
        }
    }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = Vec::new();

    for _ in 0..100 {
        let index = rng.gen::<u64>();
        let val = rng.gen::<i32>();

        runtime.function("set", SetArg(index, val).encode());

        vals.push((index, val));
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.output(), Val(val.1).encode());
    }
}

#[test]
fn test_enum() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct SetArg(u8, i32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetArg(u8);

    let mut runtime = build_solidity(
        r##"
    contract foo {
        enum bar { bar1,bar2,bar3,bar4,bar5,bar6,bar7,bar8,bar9,bar10,bar11,bar12,bar13,bar14,bar15,bar16,bar17,bar18,bar19,bar20,bar21,bar22,bar23,bar24,bar25,bar26,bar27,bar28,bar29,bar30,bar31,bar32,bar33,bar34,bar35,bar36,bar37,bar38,bar39,bar40,bar41,bar42,bar43,bar44,bar45,bar46,bar47,bar48,bar49,bar50,bar51,bar52,bar53,bar54,bar55,bar56,bar57,bar58,bar59,bar60,bar61,bar62,bar63,bar64,bar65,bar66,bar67,bar68,bar69,bar70,bar71,bar72,bar73,bar74,bar75,bar76,bar77,bar78,bar79,bar80,bar81,bar82,bar83,bar84,bar85,bar86,bar87,bar88,bar89,bar90,bar91,bar92,bar93,bar94,bar95,bar96,bar97,bar98,bar99,bar100}
        mapping(bar => int32) v;

        function set(bar index, int32 val) public {
            v[index] = val;
        }

        function get(bar index) public returns (int32) {
            return v[index];
        }
    }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = HashMap::new();

    for _ in 0..100 {
        let index = rng.gen::<u8>() % 100;
        let val = rng.gen::<i32>();

        runtime.function("set", SetArg(index, val).encode());

        vals.insert(index, val);
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.output(), Val(val.1).encode());
    }
}

#[test]
fn test_string() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(i64);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct SetArg(Vec<u8>, i64);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetArg(Vec<u8>);

    let mut runtime = build_solidity(
        r##"
    contract foo {
        mapping(bytes => int64) v;

        function set(bytes index, int64 val) public {
            v[index] = val;
        }

        function get(bytes index) public returns (int64) {
            return v[index];
        }
    }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = HashMap::new();

    for _ in 0..100 {
        let len = rng.gen::<usize>() % 256;
        let mut index = vec![0u8; len];
        rng.fill(&mut index[..]);
        let val = rng.gen::<i64>();

        runtime.function("set", SetArg(index.clone(), val).encode());

        vals.insert(index, val);
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.output(), Val(val.1).encode());
    }
}

#[test]
fn test_user() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct AddArg(Vec<u8>, [u8; 32]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetArg(Vec<u8>);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetRet(bool, [u8; 32]);

    let mut runtime = build_solidity(
        r#"
        contract b {
            struct user {
                bool exists;
                address addr;
            }
            mapping(string => user) users;

            function add(string name, address addr) public {
                // assigning to a storage variable creates a reference
                user storage s = users[name];

                s.exists = true;
                s.addr = addr;
            }

            function get(string name) public view returns (bool, address) {
                // assigning to a memory variable creates a copy
                user s = users[name];

                return (s.exists, s.addr);
            }

            function rm(string name) public {
                delete users[name];
            }

            function get_foo() public view returns (bool, address) {
                user storage s = users["foo"];

                return (s.exists, s.addr);
            }
        }"#,
    );

    let mut rng = rand::thread_rng();

    let mut vals = HashMap::new();

    for _ in 0..100 {
        let len = rng.gen::<usize>() % 256;
        let mut index = vec![0u8; len];
        rng.fill(&mut index[..]);
        let mut val = [0u8; 32];
        rng.fill(&mut val[..]);

        runtime.function("add", AddArg(index.clone(), val).encode());

        vals.insert(index, val);
    }

    for val in &vals {
        runtime.function("get", GetArg(val.0.clone()).encode());

        assert_eq!(runtime.output(), GetRet(true, *val.1).encode());
    }

    // now delete them

    for val in &vals {
        runtime.function("rm", GetArg(val.0.clone()).encode());
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.output(), GetRet(false, [0u8; 32]).encode());
    }

    runtime.function("add", AddArg(b"foo".to_vec(), [1u8; 32]).encode());

    runtime.function("get_foo", Vec::new());

    assert_eq!(runtime.output(), GetRet(true, [1u8; 32]).encode());
}

#[test]
fn test_string_map() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct AddArg([u8; 32], Vec<u8>);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetArg([u8; 32]);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetRet(Vec<u8>);

    let mut runtime = build_solidity(
        r##"
        contract b {
            struct SendTo{
                address sender;
                bytes hexOfAsset;
                bool paid;
            }

            mapping(address => SendTo) send;

            function add(address a, bytes v) public {
                send[a].hexOfAsset = v;
            }

            function get(address a) public view returns (bytes) {
                return send[a].hexOfAsset;
            }

            function rm(address a) public {
                delete send[a].hexOfAsset;
            }
        }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = HashMap::new();

    for _ in 0..10 {
        let len = rng.gen::<usize>() % 256;
        let mut val = vec![0u8; len];
        rng.fill(&mut val[..]);
        let mut address = [0u8; 32];
        rng.fill(&mut address[..]);

        runtime.function("add", AddArg(address, val.clone()).encode());

        vals.insert(address, val);
    }

    for (address, val) in &vals {
        runtime.function("get", GetArg(*address).encode());

        assert_eq!(runtime.output(), GetRet(val.clone()).encode());
    }

    // now delete them

    for address in vals.keys() {
        runtime.function("rm", GetArg(*address).encode());
    }

    for address in vals.keys() {
        runtime.function("get", GetArg(*address).encode());

        assert_eq!(runtime.output(), GetRet(Vec::new()).encode());
    }
}

#[test]
fn test_address() {
    type Address = [u8; 32];

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(i64);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct SetArg(Address, i64);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct GetArg(Address);

    let mut runtime = build_solidity(
        r##"
        contract foo {
            uint bvar1;
            uint bvar2;
            mapping(address => int64) v;

            function set(address index, int64 val) public {
                v[index] = val;
            }

            function get(address index) public returns (int64) {
                return v[index];
            }
        }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = HashMap::new();
    let mut index: Address = [0u8; 32];

    for _ in 0..100 {
        rng.fill(&mut index[..]);
        let val = rng.gen::<i64>();

        runtime.function("set", SetArg(index, val).encode());

        vals.insert(index, val);
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.output(), Val(val.1).encode());
    }
}
