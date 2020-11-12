use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use crate::{build_solidity, first_error, parse_and_resolve};
use solang::Target;

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val32(u32);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Val8(u8);

#[test]
fn parse_structs() {
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint a;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct ‘Foo’ has duplicate struct field ‘a’"
    );

    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint storage b;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "storage location ‘storage’ not allowed for struct field"
    );

    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool a;
                uint calldata b;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "storage location ‘calldata’ not allowed for struct field"
    );

    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool memory a;
                uint calldata b;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "storage location ‘memory’ not allowed for struct field"
    );

    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct definition for ‘Foo’ has no fields"
    );

    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                boolean x;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "type ‘boolean’ not found");

    // is it impossible to define recursive structs
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                Foo y;
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct ‘Foo’ has infinite size"
    );

    // is it impossible to define recursive structs
    let ns = parse_and_resolve(
        r#"
        contract c {
            s z;
        }

        struct s {
            bool f1;
            int32 f2;
            s2 f3;
        }

        struct s2 {
            bytes4 selector;
            s foo;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "struct ‘s2’ has infinite size");

    // literal initializers
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() private {
                Foo a = Foo();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct ‘Foo’ has 2 fields, not 0"
    );

    // literal initializers
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() private {
                Foo a = Foo(true, true, true);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct ‘Foo’ has 2 fields, not 3"
    );

    // literal initializers
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() private {
                Foo a = Foo({ });
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct ‘Foo’ has 2 fields, not 0"
    );

    // literal initializers
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() private {
                Foo a = Foo({ x: true, y: 1, z: 2 });
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "struct ‘Foo’ has 2 fields, not 3"
    );

    // literal initializers
    let ns = parse_and_resolve(
        r#"
        contract test_struct_parsing {
            struct Foo {
                bool x;
                int32 y;
            }

            function f() private {
                Foo a = Foo({ x: true, z: 1 });
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "struct ‘Foo’ has no field ‘z’");
}

#[test]
fn struct_members() {
    let mut runtime = build_solidity(
        r##"
        pragma solidity 0;
        pragma experimental ABIEncoderV2;

        contract test_struct_parsing {
                struct foo {
                        bool x;
                        uint32 y;
                        bytes31 d;
                }

                function test() public {
                        foo f;
                        f.x = true;
                        f.y = 64;

                        assert(f.x == true);
                        assert(f.y == 64);
                        assert(f.d.length == 31);

                        foo f2 = foo(false, 32168, hex"DEAD");

                        assert(f2.x == false);
                        assert(f2.y == 32168);
                        assert(f2.d == hex"dead");


                        foo f3 = foo({ x: true, y: 102, d: hex"00DEAD" });

                        assert(f3.x == true);
                        assert(f3.y == 102);
                        assert(f3.d == hex"00dead");
                    }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn structs_as_ref_args() {
    let mut runtime = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bool x;
                uint32 y;
            }

            function func(foo f) private {
                // assigning to f members dereferences f
                f.x = true;
                f.y = 64;

                // assigning to f changes the reference
                f = foo({ x: false, y: 256 });

                // f no longer point to f in caller function
                f.x = false;
                f.y = 98123;
            }

            function test() public {
                foo f;

                func(f);

                assert(f.x == true);
                assert(f.y == 64);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn structs_encode() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: bool,
    };

    let mut runtime = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                bool f2;
            }

            function test(foo f) public {
                assert(f.f1 == "ABC");
                assert(f.f2 == true);
            }
        }"##,
    );

    runtime.function(
        "test",
        Foo {
            f1: [0x41, 0x42, 0x43],
            f2: true,
        }
        .encode(),
    );
}

#[test]
fn structs_decode() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: i32,
    };

    let mut runtime = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                int32 f2;
            }

            function test() public returns (foo) {
                foo f;

                f.f1 = hex"f33ec3";
                f.f2 = 0xfd7f;

                return f;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.vm.output,
        Foo {
            f1: [0xf3, 0x3e, 0xc3],
            f2: 0xfd7f,
        }
        .encode(),
    );
}

#[test]
fn struct_in_struct() {
    let mut runtime = build_solidity(
        r##"
        pragma solidity 0;

        contract struct_in_struct {
            struct foo {
                bool x;
                uint32 y;
            }
            struct bar {
                address a;
                bytes7 b;
                foo c;
            }

            function test() public pure {
                bar memory f = bar({ a: address(0), b: hex"fe", c: foo({ x: true, y: 102 }) });

                foo memory m = foo(false, 50);

                f.c = m;

                f.c.y = 300;

                assert(m.y == 300);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn structs_in_structs_decode() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: i32,
    };
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bar {
        a: bool,
        b: Foo,
        c: Foo,
    };

    let mut runtime = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                int32 f2;
            }

            struct bar {
                bool a;
                foo b;
                foo c;
            }

            function test() public returns (bar) {
                bar f = bar({ a: true, b: foo({ f1: hex"c30000", f2: 0xff7f}), c: foo({ f1: hex"f7f6f5", f2: 0x4002 })});

                return f;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.vm.output,
        Bar {
            a: true,
            b: Foo {
                f1: [0xc3, 0x00, 0x00],
                f2: 0xff7f,
            },
            c: Foo {
                f1: [0xf7, 0xf6, 0xf5],
                f2: 0x4002,
            }
        }
        .encode(),
    );
}

#[test]
fn structs_in_structs_encode() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: i32,
    };
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Bar {
        a: bool,
        b: Foo,
        c: Foo,
    };

    let mut runtime = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                int32 f2;
            }

            function test(other.bar f) public {
                assert(f.c.f2 == 0x4002);
                assert(f.b.f1 == hex"c30000");
            }
        }

        contract other {
            struct bar {
                bool a;
                test_struct_parsing.foo b;
                test_struct_parsing.foo c;
            }
        }"##,
    );

    runtime.function(
        "test",
        Bar {
            a: true,
            b: Foo {
                f1: [0xc3, 0x00, 0x00],
                f2: 0xff7f,
            },
            c: Foo {
                f1: [0xf7, 0xf6, 0xf5],
                f2: 0x4002,
            },
        }
        .encode(),
    );
}

#[test]
fn struct_storage_to_memory() {
    let mut runtime = build_solidity(
        r##"
        contract test_struct_parsing {
            struct foo {
                bytes3 f1;
                int64 f2;
            }
            foo bar;

            constructor() public {
                bar.f1 = hex"123456";
                bar.f2 = 0x0123456789abcdef;
            }

            function test() public {
                foo f = bar;

                assert(f.f1 == hex"123456");
                assert(f.f2 == 81985529216486895);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());
}

#[test]
fn return_from_struct_storage() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Foo {
        f1: [u8; 3],
        f2: u32,
    };

    let mut runtime = build_solidity(
        r##"
        struct foo {
            bytes3 f1;
            uint32 f2;
        }
        contract test_struct_parsing {
            foo bar;

            constructor() public {
                bar.f1 = "png";
                bar.f2 = 0x89abcdef;
            }

            function test() public returns (foo) {
                return bar;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.vm.output,
        Foo {
            f1: [0x70, 0x6e, 0x67],
            f2: 0x89ab_cdef,
        }
        .encode(),
    );
}

#[test]
fn struct_in_init_return() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Card {
        value: u8,
        suit: u8,
    };

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Hand {
        card1: Card,
        card2: Card,
        card3: Card,
        card4: Card,
        card5: Card,
    };

    let mut runtime = build_solidity(
        r#"
        enum suit { club, diamonds, hearts, spades }
        enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
        struct card {
            value v;
            suit s;
        }
        contract structs {
            card card1 = card({ s: suit.hearts, v: value.two });
            card card2 = card({ s: suit.diamonds, v: value.three });
            card card3 = card({ s: suit.club, v: value.four });
            card card4 = card({ s: suit.diamonds, v: value.ten });
            card card5 = card({ s: suit.hearts, v: value.jack });

            function test() public {
                assert(card1.s == suit.hearts);
                assert(card1.v == value.two);
                assert(card2.s == suit.diamonds);
                assert(card2.v == value.three);
                assert(card3.s == suit.club);
                assert(card3.v == value.four);
                assert(card4.s == suit.diamonds);
                assert(card4.v == value.ten);
                assert(card5.s == suit.hearts);
                assert(card5.v == value.jack);
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());
}

#[test]
fn struct_struct_in_init_and_return() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Card {
        v: u8,
        s: u8,
    };

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Hand {
        card1: Card,
        card2: Card,
        card3: Card,
        card4: Card,
        card5: Card,
    };

    let mut runtime = build_solidity(
        r#"
        contract structs {
            enum suit { club, diamonds, hearts, spades }
            enum value { two, three, four, five, six, seven, eight, nine, ten, jack, queen, king, ace }
            struct card {
                value v;
                suit s;
            }
            struct hand {
                card card1;
                card card2;
                card card3;
                card card4;
                card card5;
            }
            hand h = hand({
                card1: card({ s: suit.hearts, v: value.two }),
                card2: card({ s: suit.diamonds, v: value.three }),
                card3: card({ s: suit.club, v: value.four }),
                card4: card({ s: suit.diamonds, v: value.ten }),
                card5: card({ s: suit.hearts, v: value.jack })
            });

            function return_struct_from_storage(hand storage n) private returns (hand) {
                return n;
            }

            function test() public {
                hand l = return_struct_from_storage(h);
                assert(l.card1.s == suit.hearts);
                assert(l.card1.v == value.two);
                assert(l.card2.s == suit.diamonds);
                assert(l.card2.v == value.three);
                assert(l.card3.s == suit.club);
                assert(l.card3.v == value.four);
                assert(l.card4.s == suit.diamonds);
                assert(l.card4.v == value.ten);
                assert(l.card5.s == suit.hearts);
                assert(l.card5.v == value.jack);
            }
        }
        "#,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());
}
