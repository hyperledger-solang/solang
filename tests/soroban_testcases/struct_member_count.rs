// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use indexmap::Equivalent;
use soroban_sdk::{testutils::Address as _, Address, FromVal, IntoVal, Val};

#[test]
fn struct_one_member() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; }
            S s;
            function set_a(uint64 v) public { s.a = v; }
            function get_a() public view returns (uint64) { return s.a; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    assert_eq!(a, 42);
}

#[test]
fn struct_two_members() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; }
            S s;
            function set_a(uint64 v) public { s.a = v; }
            function set_b(int32 v)  public { s.b = v; }
            function get_a() public view returns (uint64) { return s.a; }
            function get_b() public view returns (int32)  { return s.b; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![7_i32.into_val(&runtime.env)]);

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_b", vec![]),
    );
    assert_eq!(a, 42);
    assert_eq!(b, 7);
}

#[test]
fn struct_three_members() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; bool c; }
            S s;
            function set_a(uint64 v) public { s.a = v; }
            function set_b(int32 v)  public { s.b = v; }
            function set_c(bool v)   public { s.c = v; }
            function get_a() public view returns (uint64) { return s.a; }
            function get_b() public view returns (int32)  { return s.b; }
            function get_c() public view returns (bool)   { return s.c; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![7_i32.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_c", vec![true.into_val(&runtime.env)]);

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_b", vec![]),
    );
    let c: bool = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_c", vec![]),
    );
    assert_eq!(a, 42);
    assert_eq!(b, 7);
    assert!(c);
}

#[test]
fn struct_four_members() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; bool c; address d; }
            S s;
            function set_a(uint64 v)  public { s.a = v; }
            function set_b(int32 v)   public { s.b = v; }
            function set_c(bool v)    public { s.c = v; }
            function set_d(address v) public { s.d = v; }
            function get_a() public view returns (uint64)  { return s.a; }
            function get_b() public view returns (int32)   { return s.b; }
            function get_c() public view returns (bool)    { return s.c; }
            function get_d() public view returns (address) { return s.d; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let who = Address::generate(&runtime.env);

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![7_i32.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_c", vec![true.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_d", vec![who.clone().into_val(&runtime.env)]);

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_b", vec![]),
    );
    let c: bool = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_c", vec![]),
    );
    let d = Address::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_d", vec![]),
    );
    assert_eq!(a, 42);
    assert_eq!(b, 7);
    assert!(c);
    assert!(d.equivalent(&who));
}

#[test]
fn struct_five_members() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; bool c; address d; string e; }
            S s;
            function set_a(uint64 v)        public { s.a = v; }
            function set_b(int32 v)         public { s.b = v; }
            function set_c(bool v)          public { s.c = v; }
            function set_d(address v)       public { s.d = v; }
            function set_e(string memory v) public { s.e = v; }
            function get_a() public view returns (uint64)        { return s.a; }
            function get_b() public view returns (int32)         { return s.b; }
            function get_c() public view returns (bool)          { return s.c; }
            function get_d() public view returns (address)       { return s.d; }
            function get_e() public view returns (string memory) { return s.e; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let who = Address::generate(&runtime.env);

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![7_i32.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_c", vec![true.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_d", vec![who.clone().into_val(&runtime.env)]);
    runtime.invoke_contract(
        addr,
        "set_e",
        vec![soroban_sdk::String::from_str(&runtime.env, "hello").into_val(&runtime.env)],
    );

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_b", vec![]),
    );
    let c: bool = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_c", vec![]),
    );
    let d = Address::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_d", vec![]),
    );
    let e = soroban_sdk::String::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_e", vec![]),
    );
    assert_eq!(a, 42);
    assert_eq!(b, 7);
    assert!(c);
    assert!(d.equivalent(&who));
    assert_eq!(e, soroban_sdk::String::from_str(&runtime.env, "hello"));
}

#[test]
fn struct_six_members() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; bool c; address d; string e; bytes f; }
            S s;
            function set_a(uint64 v)        public { s.a = v; }
            function set_b(int32 v)         public { s.b = v; }
            function set_c(bool v)          public { s.c = v; }
            function set_d(address v)       public { s.d = v; }
            function set_e(string memory v) public { s.e = v; }
            function set_f(bytes memory v)  public { s.f = v; }
            function get_a() public view returns (uint64)        { return s.a; }
            function get_b() public view returns (int32)         { return s.b; }
            function get_c() public view returns (bool)          { return s.c; }
            function get_d() public view returns (address)       { return s.d; }
            function get_e() public view returns (string memory) { return s.e; }
            function get_f() public view returns (bytes memory)  { return s.f; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let who = Address::generate(&runtime.env);
    let payload: [u8; 3] = [0x01, 0x02, 0x03];

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![7_i32.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_c", vec![true.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_d", vec![who.clone().into_val(&runtime.env)]);
    runtime.invoke_contract(
        addr,
        "set_e",
        vec![soroban_sdk::String::from_str(&runtime.env, "hello").into_val(&runtime.env)],
    );
    runtime.invoke_contract(
        addr,
        "set_f",
        vec![soroban_sdk::Bytes::from_array(&runtime.env, &payload).into_val(&runtime.env)],
    );

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_b", vec![]),
    );
    let c: bool = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_c", vec![]),
    );
    let d = Address::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_d", vec![]),
    );
    let e = soroban_sdk::String::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_e", vec![]),
    );
    let f = soroban_sdk::Bytes::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_f", vec![]),
    );
    assert_eq!(a, 42);
    assert_eq!(b, 7);
    assert!(c);
    assert!(d.equivalent(&who));
    assert_eq!(e, soroban_sdk::String::from_str(&runtime.env, "hello"));
    assert_eq!(f, soroban_sdk::Bytes::from_slice(&runtime.env, &payload));
}

#[test]
fn struct_seven_members() {
    let runtime = build_solidity(
        r#"
        contract c {
            struct S { uint64 a; int32 b; bool c; address d; string e; bytes f; bytes4 g; }
            S s;
            function set_a(uint64 v)        public { s.a = v; }
            function set_b(int32 v)         public { s.b = v; }
            function set_c(bool v)          public { s.c = v; }
            function set_d(address v)       public { s.d = v; }
            function set_e(string memory v) public { s.e = v; }
            function set_f(bytes memory v)  public { s.f = v; }
            function set_g(bytes4 v)        public { s.g = v; }
            function get_a() public view returns (uint64)        { return s.a; }
            function get_b() public view returns (int32)         { return s.b; }
            function get_c() public view returns (bool)          { return s.c; }
            function get_d() public view returns (address)       { return s.d; }
            function get_e() public view returns (string memory) { return s.e; }
            function get_f() public view returns (bytes memory)  { return s.f; }
            function get_g() public view returns (bytes4)        { return s.g; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let who = Address::generate(&runtime.env);
    let payload: [u8; 3] = [0x01, 0x02, 0x03];
    let bn_payload: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];

    runtime.invoke_contract(addr, "set_a", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_b", vec![7_i32.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_c", vec![true.into_val(&runtime.env)]);
    runtime.invoke_contract(addr, "set_d", vec![who.clone().into_val(&runtime.env)]);
    runtime.invoke_contract(
        addr,
        "set_e",
        vec![soroban_sdk::String::from_str(&runtime.env, "hello").into_val(&runtime.env)],
    );
    runtime.invoke_contract(
        addr,
        "set_f",
        vec![soroban_sdk::Bytes::from_array(&runtime.env, &payload).into_val(&runtime.env)],
    );
    runtime.invoke_contract(
        addr,
        "set_g",
        vec![soroban_sdk::BytesN::from_array(&runtime.env, &bn_payload).into_val(&runtime.env)],
    );

    let a: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_a", vec![]),
    );
    let b: i32 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_b", vec![]),
    );
    let c: bool = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_c", vec![]),
    );
    let d = Address::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_d", vec![]),
    );
    let e = soroban_sdk::String::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_e", vec![]),
    );
    let f = soroban_sdk::Bytes::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_f", vec![]),
    );
    let g: soroban_sdk::BytesN<4> = soroban_sdk::BytesN::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_g", vec![]),
    );
    assert_eq!(a, 42);
    assert_eq!(b, 7);
    assert!(c);
    assert!(d.equivalent(&who));
    assert_eq!(e, soroban_sdk::String::from_str(&runtime.env, "hello"));
    assert_eq!(f, soroban_sdk::Bytes::from_slice(&runtime.env, &payload));
    assert_eq!(
        g,
        soroban_sdk::BytesN::from_array(&runtime.env, &bn_payload)
    );
}

#[test]
fn whole_struct_write_then_member_update() {
    let runtime = build_solidity(
        r#"
        contract pt {
            struct Point { uint64 x; uint64 y; }
            Point p;
            function store(uint64 x, uint64 y) public { p = Point(x, y); }
            function set_y(uint64 v) public { p.y = v; }
            function get_x() public view returns (uint64) { return p.x; }
            function get_y() public view returns (uint64) { return p.y; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(
        addr,
        "store",
        vec![3_u64.into_val(&runtime.env), 4_u64.into_val(&runtime.env)],
    );
    runtime.invoke_contract(addr, "set_y", vec![99_u64.into_val(&runtime.env)]);
    let x: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_x", vec![]),
    );
    let y: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_y", vec![]),
    );
    assert_eq!(x, 3);
    assert_eq!(y, 99);
}

#[test]
fn member_write_then_whole_struct_overwrite() {
    let runtime = build_solidity(
        r#"
        contract pt {
            struct Point { uint64 x; uint64 y; }
            Point p;
            function set_x(uint64 v) public { p.x = v; }
            function store(uint64 x, uint64 y) public { p = Point(x, y); }
            function get_x() public view returns (uint64) { return p.x; }
            function get_y() public view returns (uint64) { return p.y; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    runtime.invoke_contract(addr, "set_x", vec![42_u64.into_val(&runtime.env)]);
    runtime.invoke_contract(
        addr,
        "store",
        vec![10_u64.into_val(&runtime.env), 20_u64.into_val(&runtime.env)],
    );
    let x: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_x", vec![]),
    );
    let y: u64 = FromVal::from_val(
        &runtime.env,
        &runtime.invoke_contract(addr, "get_y", vec![]),
    );
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn struct_compound_assignment_state_var() {
    let runtime = build_solidity(
        r#"
        contract cnt {
            struct Counter { uint64 n; }
            Counter state;
            function inc(uint64 delta) public { state.n += delta; }
            function get() public view returns (uint64) { return state.n; }
        }
        "#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let zero: Val = 0_u64.into_val(&runtime.env);
    assert!(zero.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));

    runtime.invoke_contract(addr, "inc", vec![5_u64.into_val(&runtime.env)]);
    let after_first: u64 =
        FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(after_first, 5);

    runtime.invoke_contract(addr, "inc", vec![3_u64.into_val(&runtime.env)]);
    let after_second: u64 =
        FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(after_second, 8);
}
