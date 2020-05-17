use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use rand::Rng;
use std::collections::HashMap;

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[test]
fn bad_mapping_declares() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            struct s {
                uint32 x;
            mapping(uint => address) data;
            }
        
            function test() public {
                s memory x;
        
                x.data[1] = address(1);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "mapping only allowed in storage");

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            mapping(uint[] => address) data;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "key of mapping cannot be array type");

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            struct foo {
                int x;
            }
            mapping(foo => address) data;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "key of mapping cannot be struct type");

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            mapping(int => address) data;
            mapping(data => address) data2;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "‘data’ is a contract variable");

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            function test(mapping(int => address) x) public {
                //
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "parameter with mapping type must be of type ‘storage’"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            function test(mapping(int => address) storage x) public {
                //
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "parameter of type ‘storage’ not allowed public or external functions"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            function test() public returns (mapping(int => address) x) {
                //
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "return type containing mapping must be of type ‘storage’"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            function test() public returns (mapping(int => address) storage x) {
                //
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "return type of type ‘storage’ not allowed public or external functions"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            function test() public {
                int[] x = new mapping(int => address)(2);
                //
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "new cannot allocate type ‘mapping(int256 => address)’"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract c {
            mapping(uint => bool) data;
            function test() public {
                delete data;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "‘delete’ cannot be applied to mapping type"
    );
}

#[test]
fn basic() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            mapping(uint => bytes4) data;
        
            function test() public {
                data[1] = hex"cafedead";
        
                assert(data[1] == hex"cafedead");
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn test_uint64() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct SetArg(u64, i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
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

        assert_eq!(runtime.vm.scratch, Val(val.1).encode());
    }
}

#[test]
fn test_enum() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct SetArg(u8, i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
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

        assert_eq!(runtime.vm.scratch, Val(val.1).encode());
    }
}

#[test]
fn test_string() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i64);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct SetArg(Vec<u8>, i64);
    #[derive(Debug, PartialEq, Encode, Decode)]
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
        let mut index = Vec::new();
        index.resize(len, 0u8);
        rng.fill(&mut index[..]);
        let val = rng.gen::<i64>();

        runtime.function("set", SetArg(index.clone(), val).encode());

        vals.insert(index, val);
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.vm.scratch, Val(val.1).encode());
    }
}

#[test]
fn test_user() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct AddArg(Vec<u8>, [u8; 32]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct GetArg(Vec<u8>);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct GetRet(bool, [u8; 32]);

    let mut runtime = build_solidity(
        r##"
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
        }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = HashMap::new();

    for _ in 0..100 {
        let len = rng.gen::<usize>() % 256;
        let mut index = Vec::new();
        index.resize(len, 0u8);
        rng.fill(&mut index[..]);
        let mut val = [0u8; 32];
        rng.fill(&mut val[..]);

        runtime.function("add", AddArg(index.clone(), val).encode());

        vals.insert(index, val);
    }

    for val in &vals {
        runtime.function("get", GetArg(val.0.clone()).encode());

        assert_eq!(runtime.vm.scratch, GetRet(true, *val.1).encode());
    }

    // now delete them

    for val in &vals {
        runtime.function("rm", GetArg(val.0.clone()).encode());
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.vm.scratch, GetRet(false, [0u8; 32]).encode());
    }
}
