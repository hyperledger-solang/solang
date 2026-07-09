// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal};

#[test]
fn whole_write_then_copy_read_two_fields() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; uint64 b; }
            S s;
            function store(uint64 a, uint64 b) public { s = S(a, b); }
            function copy_a() public view returns (uint64) {
                S memory copy = s;
                return copy.a;
            }
            function copy_b() public view returns (uint64) {
                S memory copy = s;
                return copy.b;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(
        addr,
        "store",
        vec![10_u64.into_val(&runtime.env), 20_u64.into_val(&runtime.env)],
    );
    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_a", vec![]),
    );
    let b: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_b", vec![]),
    );
    assert_eq!(a, 10);
    assert_eq!(b, 20);
}

#[test]
fn whole_copy_read_string_bytes_bytesn() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { string a; bytes b; bytes4 c; }
            S s;
            function set_a(string memory v)  public { s.a = v; }
            function set_b(bytes memory v)   public { s.b = v; }
            function set_c(bytes4 v)         public { s.c = v; }
            function copy_a() public view returns (string memory) {
                S memory copy = s;
                return copy.a;
            }
            function copy_b() public view returns (bytes memory) {
                S memory copy = s;
                return copy.b;
            }
            function copy_c() public view returns (bytes4) {
                S memory copy = s;
                return copy.c;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let payload: [u8; 3] = [0x01, 0x02, 0x03];
    let bn_payload: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];

    runtime.invoke_contract(
        addr,
        "set_a",
        vec![soroban_sdk::String::from_str(&runtime.env, "hello").into_val(&runtime.env)],
    );
    runtime.invoke_contract(
        addr,
        "set_b",
        vec![soroban_sdk::Bytes::from_array(&runtime.env, &payload).into_val(&runtime.env)],
    );
    runtime.invoke_contract(
        addr,
        "set_c",
        vec![soroban_sdk::BytesN::from_array(&runtime.env, &bn_payload).into_val(&runtime.env)],
    );

    let a = soroban_sdk::String::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_a", vec![]),
    );
    let b = soroban_sdk::Bytes::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_b", vec![]),
    );
    let c: soroban_sdk::BytesN<4> = soroban_sdk::BytesN::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_c", vec![]),
    );

    assert_eq!(a, soroban_sdk::String::from_str(&runtime.env, "hello"));
    assert_eq!(b, soroban_sdk::Bytes::from_slice(&runtime.env, &payload));
    assert_eq!(
        c,
        soroban_sdk::BytesN::from_array(&runtime.env, &bn_payload)
    );
}

#[test]
fn member_writes_then_copy_read() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; bool c; }
            S s;
            function set_a(uint64 v) public { s.a = v; }
            function set_b(int32 v)  public { s.b = v; }
            function set_c(bool v)   public { s.c = v; }
            function copy_a() public view returns (uint64) {
                S memory copy = s;
                return copy.a;
            }
            function copy_b() public view returns (int32) {
                S memory copy = s;
                return copy.b;
            }
            function copy_c() public view returns (bool) {
                S memory copy = s;
                return copy.c;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(addr, "set_a", vec![99_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![3_i32.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_c", vec![true.into_val(&runtime.env)]);
    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_b", vec![]),
    );
    let c: bool = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_c", vec![]),
    );
    assert_eq!(a, 99);
    assert_eq!(b, 3);
    assert!(c);
}

#[test]
fn double_whole_struct_write() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 x; uint64 y; }
            S s;
            function store(uint64 x, uint64 y) public { s = S(x, y); }
            function get_x() public view returns (uint64) { return s.x; }
            function get_y() public view returns (uint64) { return s.y; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(
        addr,
        "store",
        vec![1_u64.into_val(&runtime.env), 2_u64.into_val(&runtime.env)],
    );
    runtime.invoke_contract(
        addr,
        "store",
        vec![
            100_u64.into_val(&runtime.env),
            200_u64.into_val(&runtime.env),
        ],
    );
    let x: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_x", vec![]),
    );
    let y: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_y", vec![]),
    );
    assert_eq!(x, 100);
    assert_eq!(y, 200);
}

#[test]
fn copy_read_then_compute() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; uint64 b; uint64 c; }
            S s;
            function store(uint64 a, uint64 b, uint64 c) public { s = S(a, b, c); }
            function sum() public view returns (uint64) {
                S memory copy = s;
                return copy.a + copy.b + copy.c;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(
        addr,
        "store",
        vec![
            10_u64.into_val(&runtime.env),
            20_u64.into_val(&runtime.env),
            30_u64.into_val(&runtime.env),
        ],
    );
    let total: u64 = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "sum", vec![]));
    assert_eq!(total, 60);
}

#[test]
fn two_structs_in_storage_dont_interfere() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct A { uint64 x; }
            struct B { uint64 y; }
            A a;
            B b;
            function store_a(uint64 v) public { a = A(v); }
            function store_b(uint64 v) public { b = B(v); }
            function copy_a() public view returns (uint64) {
                A memory ca = a;
                return ca.x;
            }
            function copy_b() public view returns (uint64) {
                B memory cb = b;
                return cb.y;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(addr, "store_a", vec![11_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "store_b", vec![22_u64.into_val(&runtime.env)]);
    let av: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_a", vec![]),
    );
    let bv: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_b", vec![]),
    );
    assert_eq!(av, 11);
    assert_eq!(bv, 22);

    runtime.invoke_contract(addr, "store_a", vec![99_u64.into_val(&runtime.env)]);
    let av2: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_a", vec![]),
    );
    let bv2: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_b", vec![]),
    );
    assert_eq!(av2, 99);
    assert_eq!(bv2, 22);
}

#[test]
fn whole_write_then_partial_member_update_copy_read() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 x; uint64 y; uint64 z; }
            S s;
            function store(uint64 x, uint64 y, uint64 z) public { s = S(x, y, z); }
            function update_y(uint64 v) public { s.y = v; }
            function copy_x() public view returns (uint64) {
                S memory copy = s;
                return copy.x;
            }
            function copy_y() public view returns (uint64) {
                S memory copy = s;
                return copy.y;
            }
            function copy_z() public view returns (uint64) {
                S memory copy = s;
                return copy.z;
            }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(
        addr,
        "store",
        vec![
            1_u64.into_val(&runtime.env),
            2_u64.into_val(&runtime.env),
            3_u64.into_val(&runtime.env),
        ],
    );
    runtime.invoke_contract(addr, "update_y", vec![99_u64.into_val(&runtime.env)]);
    let x: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_x", vec![]),
    );
    let y: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_y", vec![]),
    );
    let z: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "copy_z", vec![]),
    );
    assert_eq!(x, 1);
    assert_eq!(y, 99);
    assert_eq!(z, 3);
}
