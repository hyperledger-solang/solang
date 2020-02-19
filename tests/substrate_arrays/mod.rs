use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use rand::Rng;

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val32(u32);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val8(u8);

#[test]
fn missing_array_index() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public returns (uint) {
                    uint8[4] memory bar = [ 1, 2, 3, 4 ];

                    return bar[];
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "expected expression before ‘]’ token");

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function foo() public returns (uint8) {
                    uint8[4] memory bar = [ 1, 2, 3, 4, 5 ];

                    return bar[0];
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "conversion from uint8[5] to uint8[4] not possible"
    );
}

#[test]
fn const_array_array() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            int8[8] constant bar = [ int8(1), 2, 3, 4, 5, 6, 7, 8 ];

            function f(uint32 i1) public returns (int8) {
                return bar[i1];
            }
        }"##,
    );

    runtime.function(&mut store, "f", Val32(1).encode());

    assert_eq!(store.scratch, Val8(2).encode());
}

#[test]
fn votes() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Votes([bool; 11]);

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            /// In a jury, do the ayes have it?
            function f(bool[11] votes) public pure returns (bool) {
                    uint32 i;
                    uint32 ayes = 0;
    
                    for (i=0; i<votes.length; i++) {
                            if (votes[i]) {
                                    ayes += 1;
                            }
                    }
    
                    return ayes > votes.length / 2;
            }
        }"##,
    );

    runtime.function(
        &mut store,
        "f",
        Votes([
            true, true, true, true, true, true, false, false, false, false, false,
        ])
        .encode(),
    );

    assert_eq!(store.scratch, true.encode());

    runtime.function(
        &mut store,
        "f",
        Votes([
            true, true, true, true, true, false, false, false, false, false, false,
        ])
        .encode(),
    );

    assert_eq!(store.scratch, false.encode());
}

#[test]
fn return_array() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Res([u64; 4]);

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            function array() pure public returns (int64[4]) {
                return [ int64(4), 84564, 31213, 1312 ];
        }
        }"##,
    );

    runtime.function(&mut store, "array", Vec::new());

    assert_eq!(store.scratch, Res([4, 84564, 31213, 1312]).encode());
}

#[test]
fn storage_arrays() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct SetArg(u64, i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct GetArg(u64);

    let (runtime, mut store) = build_solidity(
        r##"
        contract foo {
            int32[8589934592] bigarray;

            function set(uint64 index, int32 val) public {
                bigarray[index] = val;
            }

            function get(uint64 index) public returns (int32) {
                return bigarray[index];
            }
        }"##,
    );

    let mut rng = rand::thread_rng();

    let mut vals = Vec::new();

    for _ in 0..100 {
        let index = rng.gen::<u64>() % 0x2_000_000;
        let val = rng.gen::<i32>();

        runtime.function(&mut store, "set", SetArg(index, val).encode());

        vals.push((index, val));
    }

    for val in vals {
        runtime.function(&mut store, "get", GetArg(val.0).encode());

        assert_eq!(store.scratch, Val(val.1).encode());
    }
}

#[test]
fn enum_arrays() {
    #[derive(Encode, Decode)]
    struct Arg([u8; 100]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(u32);

    let (runtime, mut store) = build_solidity(
        r##"
        contract enum_array {
            enum Foo { Bar1, Bar2, Bar3, Bar4 }
            
            function count_bar2(Foo[100] calldata foos) public returns (uint32) {
                uint32 count = 0;
                uint32 i;
        
                for (i = 0; i < foos.length; i++) {
                    if (foos[i] == Foo.Bar2) {
                        count++;
                    }
                }
        
                return count;
            }
        }"##,
    );

    let mut rng = rand::thread_rng();

    let mut a = [0u8; 100];
    let mut count = 0;

    #[allow(clippy::needless_range_loop)]
    for i in 0..a.len() {
        a[i] = rng.gen::<u8>() % 4;
        if a[i] == 1 {
            count += 1;
        }
    }

    runtime.function(&mut store, "count_bar2", Arg(a).encode());
    assert_eq!(store.scratch, Ret(count).encode());
}

#[test]
fn data_locations() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function bar(uint storage) public returns () {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "data location ‘storage’ can only be specified for array, struct or mapping"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            function bar(uint calldata x) public returns () {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "data location ‘calldata’ can only be specified for array, struct or mapping"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 memory x) public returns () {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "data location ‘memory’ can only be specified for array, struct or mapping"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (uint calldata) {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "data location ‘calldata’ can only be specified for array, struct or mapping"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (bool calldata) {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "data location ‘calldata’ can only be specified for array, struct or mapping"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (int storage) {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "data location ‘storage’ can only be specified for array, struct or mapping"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(int[10] storage x) public returns (int) {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "parameter of type ‘storage’ not allowed public or external functions"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (int[10] storage x) {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "return type of type ‘storage’ not allowed public or external functions"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (foo2[10] storage x) {
            }
        }"#,
        &Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "return type of type ‘storage’ not allowed public or external functions"
    );
}

#[test]
fn storage_ref_arg() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract storage_refs {
            int32[10] a;
            int32[10] b;
        
            function set(int32[10] storage array, uint8 index, int32 val) private {
                array[index] = val;
            }
        
            function test() public {
                set(a, 2, 5);
                set(b, 2, 102);
        
                assert(a[2] == 5);
                assert(b[2] == 102);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}

#[test]
fn storage_ref_var() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract storage_refs {
            int32[10] a;
            int32[10] b;
        
            function set(int32[10] storage array, uint8 index, int32 val) private {
                array[index] = val;
            }
        
            function test() public {
                int32[10] storage ref = a;
        
                set(ref, 2, 5);
        
                ref = b;
        
                set(ref, 2, 102);
        
                assert(a[2] == 5);
                assert(b[2] == 102);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}

#[test]
fn storage_ref_returns() {
    let (runtime, mut store) = build_solidity(
        r##"
        contract storage_refs {
            int32[10] a;
            int32[10] b;
        
            function a_or_b(bool want_a) private returns (int32[10] storage) {
                if (want_a) {
                    return a;
                }
        
                return b;
            }
        
            function set(int32[10] storage array, uint8 index, int32 val) private {
                array[index] = val;
            }
        
            function test() public {
                int32[10] storage ref = a_or_b(true);
        
                set(ref, 2, 5);
        
                ref = a_or_b(false);
        
                set(ref, 2, 102);
        
                assert(a[2] == 5);
                assert(b[2] == 102);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}

#[test]
fn storage_to_memory() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret([u32; 10]);

    let (runtime, mut store) = build_solidity(
        r##"
        contract storage_refs {
            int32[10] a;
        
            function test() public returns (int32[10]) {
                for (int32 i  = 0; i < 10; i++) {
                    a[i] = 7 * (i + 1);
                }

                return a;
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());

    let val = Ret([7, 14, 21, 28, 35, 42, 49, 56, 63, 70]);

    assert_eq!(store.scratch, val.encode());
}

#[test]
fn array_dimensions() {
    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            bool[10 - 10] x;
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "zero size of array declared");

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            bool[-10 + 10] x;
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "zero size of array declared");

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            bool[1 / 10] x;
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "zero size of array declared");

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            enum e { e1, e2, e3 }
            e[1 / 0] x;
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "divide by zero");

    let (_, errors) = parse_and_resolve(
        r#"
        contract foo {
            struct bar { 
                int32 x;
            }
            bar[1 % 0] x;
        }"#,
        &Target::Substrate,
    );

    assert_eq!(first_error(errors), "divide by zero");

    let (runtime, mut store) = build_solidity(
        r##"
        contract storage_refs {
            int32[2**16] a;
        
            function test() public {
                assert(a.length == 65536);
            }
        }"##,
    );

    runtime.function(&mut store, "test", Vec::new());
}
