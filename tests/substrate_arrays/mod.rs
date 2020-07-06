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
    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public returns (uint) {
                    uint8[4] memory bar = [ 1, 2, 3, 4 ];

                    return bar[];
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expected expression before ‘]’ token"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            function foo() public returns (uint8) {
                    uint8[4] memory bar = [ 1, 2, 3, 4, 5 ];

                    return bar[0];
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from uint8[5] to uint8[4] not possible"
    );
}

#[test]
fn const_array_array() {
    let mut runtime = build_solidity(
        r##"
        contract foo {
            int8[8] constant bar = [ int8(1), 2, 3, 4, 5, 6, 7, 8 ];

            function f(uint32 i1) public returns (int8) {
                return bar[i1];
            }
        }"##,
    );

    runtime.function("f", Val32(1).encode());

    assert_eq!(runtime.vm.scratch, Val8(2).encode());
}

#[test]
fn votes() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Votes([bool; 11]);

    let mut runtime = build_solidity(
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
        "f",
        Votes([
            true, true, true, true, true, true, false, false, false, false, false,
        ])
        .encode(),
    );

    assert_eq!(runtime.vm.scratch, true.encode());

    runtime.function(
        "f",
        Votes([
            true, true, true, true, true, false, false, false, false, false, false,
        ])
        .encode(),
    );

    assert_eq!(runtime.vm.scratch, false.encode());
}

#[test]
fn return_array() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Res([u64; 4]);

    let mut runtime = build_solidity(
        r##"
        contract foo {
            function array() pure public returns (int64[4]) {
                return [ int64(4), 84564, 31213, 1312 ];
        }
        }"##,
    );

    runtime.function("array", Vec::new());

    assert_eq!(runtime.vm.scratch, Res([4, 84564, 31213, 1312]).encode());
}

#[test]
fn storage_arrays() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct SetArg(u64, i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct GetArg(u64);

    let mut runtime = build_solidity(
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

        runtime.function("set", SetArg(index, val).encode());

        vals.push((index, val));
    }

    for val in vals {
        runtime.function("get", GetArg(val.0).encode());

        assert_eq!(runtime.vm.scratch, Val(val.1).encode());
    }
}

#[test]
fn enum_arrays() {
    #[derive(Encode, Decode)]
    struct Arg([u8; 100]);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
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

    runtime.function("count_bar2", Arg(a).encode());
    assert_eq!(runtime.vm.scratch, Ret(count).encode());
}

#[test]
fn data_locations() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            function bar(uint storage) public returns () {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "data location ‘storage’ can only be specified for array, struct or mapping"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function bar(uint calldata x) public returns () {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "data location ‘calldata’ can only be specified for array, struct or mapping"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 memory x) public returns () {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "data location ‘memory’ can only be specified for array, struct or mapping"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (uint calldata) {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "data location ‘calldata’ can only be specified for array, struct or mapping"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (bool calldata) {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "data location ‘calldata’ can only be specified for array, struct or mapping"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(foo2 x) public returns (int storage) {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "data location ‘storage’ can only be specified for array, struct or mapping"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar(int[10] storage x) public returns (int) {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "parameter of type ‘storage’ not allowed public or external functions"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (int[10] storage x) {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "return type of type ‘storage’ not allowed public or external functions"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum foo2 { bar1, bar2 }
            function bar() public returns (foo2[10] storage x) {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "return type of type ‘storage’ not allowed public or external functions"
    );
}

#[test]
fn storage_ref_arg() {
    let mut runtime = build_solidity(
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

    runtime.function("test", Vec::new());
}

#[test]
fn storage_ref_var() {
    let mut runtime = build_solidity(
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

    runtime.function("test", Vec::new());
}

#[test]
fn storage_ref_returns() {
    let mut runtime = build_solidity(
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

    runtime.function("test", Vec::new());
}

#[test]
fn storage_to_memory() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret([u32; 10]);

    let mut runtime = build_solidity(
        r##"
        contract storage_refs {
            uint32[10] a;
        
            function test() public returns (uint32[10]) {
                for (uint32 i  = 0; i < 10; ) {
                    uint32 index = i;
                    a[index] = 7 * ++i;
                }

                return a;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let val = Ret([7, 14, 21, 28, 35, 42, 49, 56, 63, 70]);

    assert_eq!(runtime.vm.scratch, val.encode());
}

#[test]
fn memory_to_storage() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret([u32; 10]);

    let mut runtime = build_solidity(
        r##"
        contract storage_refs {
            int32[10] a;
        
            function test() public returns (int32[10]) {
                int32[10] b = [ int32(7), 14, 21, 28, 35, 42, 49, 56, 63, 70 ];

                a = b;

                return a;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let val = Ret([7, 14, 21, 28, 35, 42, 49, 56, 63, 70]);

    assert_eq!(runtime.vm.scratch, val.encode());
}

#[test]
fn array_dimensions() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            bool[10 - 10] x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "zero size array not permitted");

    let ns = parse_and_resolve(
        r#"
        contract foo {
            bool[-10 + 10] x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "zero size array not permitted");

    let ns = parse_and_resolve(
        r#"
        contract foo {
            bool[1 / 10] x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "zero size array not permitted");

    let ns = parse_and_resolve(
        r#"
        contract foo {
            enum e { e1, e2, e3 }
            e[1 / 0] x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "divide by zero");

    let ns = parse_and_resolve(
        r#"
        contract foo {
            struct bar { 
                int32 x;
            }
            bar[1 % 0] x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "divide by zero");

    let mut runtime = build_solidity(
        r##"
        contract storage_refs {
            int32[2**16] a;
        
            function test() public {
                assert(a.length == 65536);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn array_in_struct() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret([u32; 10]);

    let mut runtime = build_solidity(
        r##"
        contract storage_refs {
            struct foo {
                int32[10] f1;
            }
        
            function test() public returns (int32[10]) {
                foo a = foo({f1: [ int32(7), 14, 21, 28, 35, 42, 49, 56, 63, 0 ]});
                assert(a.f1[1] == 14);
                a.f1[9] = 70;
                return a.f1;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let val = Ret([7, 14, 21, 28, 35, 42, 49, 56, 63, 70]);

    assert_eq!(runtime.vm.scratch, val.encode());
}

#[test]
fn struct_array_struct() {
    let mut runtime = build_solidity(
        r##"
        contract flipper {
            struct bool_struct {
                bool foo_bool;
            }
        
            struct struct_bool_struct_array {
                bool_struct[1] foo_struct_array;
            }
    
            function get_memory() public pure returns (bool) {
                bool_struct memory foo = bool_struct({foo_bool: true});
                bool_struct[1] memory foo_array = [foo];
                struct_bool_struct_array memory foo_struct = struct_bool_struct_array({foo_struct_array: foo_array});
        
                /* return foo_array[0].foo_bool; */
                return foo_struct.foo_struct_array[0].foo_bool;
            }
        }"##,
    );

    runtime.function("get_memory", Vec::new());

    assert_eq!(runtime.vm.scratch, true.encode());
}

#[test]
fn struct_array_struct_abi() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: u32,
        f2: bool,
    }

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bar {
        bars: [Foo; 10],
    }

    let mut runtime = build_solidity(
        r##"
        contract flipper {
            struct foo {
                int32 f1;
                bool f2;
            }
        
            struct bar  {
                foo[10] bars;
            }

            bar s;
    
            function get_bar() public returns (bar) {
                bar memory a = bar({ bars: [
                    foo({ f1: 1, f2: true}), 
                    foo({ f1: 2, f2: true}), 
                    foo({ f1: 3, f2: true}), 
                    foo({ f1: 4, f2: true}), 
                    foo({ f1: 5, f2: true}), 
                    foo({ f1: 6, f2: true}), 
                    foo({ f1: 7, f2: false}), 
                    foo({ f1: 8, f2: true}), 
                    foo({ f1: 9, f2: true}), 
                    foo({ f1: 10, f2: true})
                ]});

                s = a;
                
                return a;
            }

            function set_bar(bar memory a) public {
                for (uint32 i = 0; i < 10; i++) {
                    assert(a.bars[i].f1 == i + 1);
                    assert(a.bars[i].f2 == (i != 6));
                }

                a = s;
            }
        }"##,
    );

    let b = Bar {
        bars: [
            Foo { f1: 1, f2: true },
            Foo { f1: 2, f2: true },
            Foo { f1: 3, f2: true },
            Foo { f1: 4, f2: true },
            Foo { f1: 5, f2: true },
            Foo { f1: 6, f2: true },
            Foo { f1: 7, f2: false },
            Foo { f1: 8, f2: true },
            Foo { f1: 9, f2: true },
            Foo { f1: 10, f2: true },
        ],
    };

    runtime.function("get_bar", Vec::new());

    assert_eq!(runtime.vm.scratch, b.encode());

    runtime.function("set_bar", b.encode());
}

#[test]
fn memory_dynamic_array_new() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[]();

                assert(a.length == 5);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new dynamic array should have a single length argument"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](1, 2);

                assert(a.length == 5);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new dynamic array should have a single length argument"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](hex"ab");

                assert(a.length == 5);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new size argument must be unsigned integer, not ‘bytes1’"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](-1);

                assert(a.length == 5);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new size argument must be unsigned integer, not ‘int8’"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new bool(1);

                assert(a.length == 5);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new cannot allocate type ‘bool’"
    );

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](5);

                assert(a.length == 5);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    // The Ethereum Foundation solc allows you to create arrays of length 0
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                bool[] memory a = new bool[](0);

                assert(a.length == 0);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn memory_dynamic_array_deref() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);

                a[-1] = 5;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "array subscript must be an unsigned integer, not ‘int8’"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](2);
                int32 i = 1;

                a[i] = 5;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "array subscript must be an unsigned integer, not ‘int32’"
    );

    // The Ethereum Foundation solc allows you to create arrays of length 0
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](5);

                assert(a.length == 5);
                a[0] = 102;
                a[1] = -5;
                a[4] = 0x7cafeeed;

                assert(a[0] == 102);
                assert(a[1] == -5);
                assert(a[4] == 0x7cafeeed);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
#[should_panic]
fn array_bounds_dynamic_array() {
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                int32[] memory a = new int32[](5);

                a[5] = 102;
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
#[should_panic]
fn empty_array_bounds_dynamic_array() {
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                bytes32[] memory a = new bytes32[](0);

                a[0] = "yo";
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn memory_dynamic_array_types_call_return() {
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function a(bool cond) public returns (bytes27[]) {
                bytes27[] foo;
                foo = new bytes27[](5);
                foo[1] = "cond was true";
                return foo;
            }

            function b(bytes27[] x) private {
                x[1] = "b was called";

                x = new bytes27[](3);
                x[1] = "should not be";
            }

            function test() public {
                bytes27[] x = a(true);
                assert(x.length == 5);
                assert(x[1] == "cond was true");

                b(x);
                assert(x.length == 5);
                assert(x[1] == "b was called");
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn dynamic_arrays_need_phi_node() {
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            enum bar { bar1, bar2, bar3, bar4 }

            function a(bool cond) public returns (bar[] memory) {
                bar[] memory foo;
                if (cond) {
                    foo = new bar[](5);
                    foo[1] = bar.bar2;
                } else {
                    foo = new bar[](3);
                    foo[1] = bar.bar3;
                }
                return foo;
            }

            function test() public {
                bar[] memory x = a(true);
                assert(x.length == 5);
                assert(x[1] == bar.bar2);

                x = a(false);
                assert(x.length == 3);
                assert(x[1] == bar.bar3);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

// test:
// alignment of array elements
// arrays of other structs/arrays/darrays
// nil pointer should fail

// copy to/from storage <=> memory
// abi encode/decode

// string/bytes

#[test]
fn storage_dynamic_array_length() {
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn storage_dynamic_array_push() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.push(102, 20);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘push()’ takes at most 1 argument"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            int32[4] bar;

            function test() public {
                bar.push(102);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘push()’ not allowed on fixed length array"
    );

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.push(102);
                assert(bar[0] == 102);
                assert(bar.length == 1);
                bar.push();
                assert(bar[1] == 0);
                assert(bar.length == 2);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    // push() returns a reference to the thing
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            struct s {
                int32 f1;
                bool f2;
            }
            s[] bar;

            function test() public {
                s storage n = bar.push();
                n.f1 = 102;
                n.f2 = true;

                assert(bar[0].f1 == 102);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let ns = parse_and_resolve(
        r#"
        contract foo {
            struct s {
                int32 f1;
                bool f2;
            }
            s[] bar;

            function test() public {
                s storage n = bar.push(s(-1, false));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function or method does not return a value"
    );
}

#[test]
fn storage_dynamic_array_pop() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.pop(102);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘pop()’ does not take any arguments"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            int32[4] bar;

            function test() public {
                bar.pop();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "method ‘pop()’ not allowed on fixed length array"
    );

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            int32[] bar;

            function test() public {
                assert(bar.length == 0);
                bar.push(102);
                assert(bar[0] == 102);
                assert(bar.length == 1);
                int32 v = bar.pop();
                assert(bar.length == 0);
                assert(v == 102);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    // We should have one entry for the length; pop should have removed the 102 entry
    assert_eq!(runtime.store.len(), 1);

    // ensure that structs and fixed arrays are wiped by pop
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            enum enum1 { val1, val2, val3 }
            struct s {
                bool f1;
                bytes3 f2;
                enum1 f3;
                uint64 f4;
                int64[2] f5;
            }
            s[] bar;

            function test() public {
                s storage first = bar.push();

                first.f1 = true;
                first.f2 = "abc";
                first.f3 = enum1.val2;
                first.f4 = 65536;
                first.f5[0] = -1;
                first.f5[1] = 5;

                assert(bar[0].f5[1] == 5);

                // now erase it again
                bar.pop();
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    // We should have one entry for the length; pop should have removed the 102 entry
    assert_eq!(runtime.store.len(), 1);

    // pop returns the dereferenced value, not a reference to storage
    let ns = parse_and_resolve(
        r#"
        contract foo {
            struct s {
                bool f1;
                int32 f2;
            }
            s[] bar;

            function test() public {
                s storage x = bar.pop();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from struct foo.s to struct foo.s storage not possible"
    );
}

#[test]
fn storage_delete() {
    let ns = parse_and_resolve(
        r#"
        contract foo {
            int32[] bar;

            function test() public {
                delete 102;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "argument to ‘delete’ should be storage reference"
    );

    let ns = parse_and_resolve(
        r#"
        contract foo {
            int32[] bar;

            function test() public {
                int32 x = delete bar;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "delete not allowed in expression"
    );

    // ensure that structs and fixed arrays are wiped by pop
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            uint64 bar;

            function test() public {
                bar = 0xdeaddeaddeaddead;

                delete bar;
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    // We should have one entry for the length; pop should have removed the 102 entry
    assert!(runtime.store.is_empty());

    // ensure that structs and fixed arrays are wiped by delete
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            enum enum1 { val1, val2, val3 }
            struct s {
                bool f1;
                bytes3 f2;
                enum1 f3;
                uint64 f4;
                int64[2] f5;
            }
            s[] bar;

            function test() public {
                s storage first = bar.push();

                first.f1 = true;
                first.f2 = "abc";
                first.f3 = enum1.val2;
                first.f4 = 65536;
                first.f5[0] = -1;
                first.f5[1] = 5;

                assert(bar[0].f5[1] == 5);

                // now erase it again
                delete bar[0];
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    // We should have one entry for the length; delete should have removed the entry
    assert_eq!(runtime.store.len(), 1);

    // ensure that structs and fixed arrays are wiped by delete
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            int[] bar;

            function setup() public {
                bar.push(102);
                bar.push(305);
            }

            function clear() public {
                delete bar;
            }
        }"#,
    );

    runtime.function("setup", Vec::new());

    assert_eq!(runtime.store.len(), 3);

    runtime.function("clear", Vec::new());

    assert_eq!(runtime.store.len(), 0);

    // our delete operator has to iterate over the dynamic array. Ensure it works if the array is empty
    runtime.function("clear", Vec::new());

    assert_eq!(runtime.store.len(), 0);
}

#[test]
fn storage_dynamic_copy() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            int32[] foo;
        
            constructor() public {
                for (int32 i = 0; i <11; i++) {
                    foo.push(i * 3);
                }
            }
        
            function storage_to_memory() view public {
                int32[] memory bar = foo;
        
                assert(bar.length == 11);
        
                for (int32 i = 0; i <11; i++) {
                    assert(bar[uint32(i)] == i * 3);
                }
            }

            function memory_to_storage() public {
                int32[] memory bar = new int32[](5);
        
                for (int32 i = 0; i < 5; i++) {
                    bar[uint32(i)] = 5 * (i + 7);
                }
        
                foo = bar;

                assert(foo.length == 5);

                for (int32 i = 0; i < 5; i++) {
                    assert(foo[uint32(i)] == 5 * (i + 7));
                }
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("storage_to_memory", Vec::new());
    runtime.function("memory_to_storage", Vec::new());

    assert_eq!(runtime.store.len(), 6);
}

#[test]
fn abi_encode_dynamic_array() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Int32Array(Vec<i32>);

    let mut runtime = build_solidity(
        r#"
        contract c {
            function encode() pure public returns (int32[]) {
                int32[] memory bar = new int32[](11);
        
                for (int32 i = 0; i <11; i++) {
                    bar[uint32(i)] = i * 3;
                }

                return bar;
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("encode", Vec::new());

    assert_eq!(
        runtime.vm.scratch,
        Int32Array(vec!(0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30)).encode()
    );
}

#[test]
fn abi_decode_dynamic_array() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Int32Array(Vec<i32>);

    let mut runtime = build_solidity(
        r#"
        contract c {
            function decode(int32[] bar) pure public {
                assert(bar.length == 11);

                for (int32 i = 0; i <11; i++) {
                    assert(bar[uint32(i)] == i * 3);
                }
            }

            function decode_empty(int32[] bar) pure public {
                assert(bar.length == 0);
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function(
        "decode",
        Int32Array(vec![0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30]).encode(),
    );

    runtime.function("decode_empty", Int32Array(vec![]).encode());
}

#[test]
fn abi_encode_dynamic_array2() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Array(Vec<(bool, u32)>);

    let mut runtime = build_solidity(
        r#"
        contract structs {
            struct foo {
                bool x;
                uint32 y;
            }
        
            function test() public returns (foo[]) {
                foo[] x = new foo[](3);

                x[0] = foo({x: true, y: 64});
                x[1] = foo({x: false, y: 102});
                x[2] = foo({x: true, y: 0x800});

                return x;
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.vm.scratch,
        Array(vec!((true, 64), (false, 102), (true, 0x800))).encode()
    );
}

#[test]
fn abi_encode_dynamic_array3() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Array(Vec<(bool, u32, String)>);

    let mut runtime = build_solidity(
        r#"
        contract structs {
            struct foo {
                bool x;
                uint32 y;
                string z;
            }
        
            function test() public returns (foo[]) {
                foo[] x = new foo[](3);

                x[0] = foo({x: true, y: 64, z: "abc"});
                x[1] = foo({x: false, y: 102, z: "a"});
                x[2] = foo({x: true, y: 0x800, z: "abcdef"});

                return x;
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.vm.scratch,
        Array(vec!(
            (true, 64, "abc".to_owned()),
            (false, 102, "a".to_owned()),
            (true, 0x800, "abcdef".to_owned())
        ))
        .encode()
    );
}

#[test]
fn abi_encode_dynamic_array4() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Array([(bool, u32, String); 3]);

    let mut runtime = build_solidity(
        r#"
        contract structs {
            struct foo {
                bool x;
                uint32 y;
                string z;
            }
        
            function test() public returns (foo[3]) {
                foo[3] x;
                x[0] = foo({x: true, y: 64, z: "abc"});
                x[1] = foo({x: false, y: 102, z: "a"});
                x[2] = foo({x: true, y: 0x800, z: "abcdef"});
                return x;
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test", Vec::new());
    runtime.heap_verify();

    assert_eq!(
        runtime.vm.scratch,
        Array([
            (true, 64, "abc".to_owned()),
            (false, 102, "a".to_owned()),
            (true, 0x800, "abcdef".to_owned())
        ])
        .encode()
    );
}
