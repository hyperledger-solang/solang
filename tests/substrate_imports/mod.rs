extern crate solang;

use super::{first_error, no_errors};
use solang::file_cache::FileCache;
use solang::Target;

#[test]
fn enum_import() {
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol";

        contract foo {
            enum_b bar;
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        enum enum_b { b1 }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import { enum_b } from "b.sol";

        contract foo {
            enum_b bar;
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        enum enum_b { b1 }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import { enum_b as foobar } from "b.sol";

        contract foo {
            foobar bar;
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        enum enum_b { b1 }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import { enum_c } from "b.sol";
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        enum enum_b { b1 }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    assert_eq!(
        first_error(ns.diagnostics),
        "import ‘b.sol’ does not export ‘enum_c’"
    );
}

#[test]
fn struct_import() {
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol";

        struct foo {
            struct_a bar;
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        struct struct_a { uint32 f1; }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import { struct_a as not_struct_a } from "b.sol";

        struct foo {
            struct_a bar;
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        struct struct_a { uint32 f1; }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    assert_eq!(first_error(ns.diagnostics), "type ‘struct_a’ not found");
}

#[test]
fn contract_import() {
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol";

        contract a {
            function go() public {
                b x = new b();

                assert(x.test() == 102);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        contract b {
            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    // lets try a importing an import
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol";

        contract a {
            function go() public {
                c x = new c();

                assert(x.test() == 102);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        import "c.sol";
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "c.sol".to_string(),
        r#"
        contract c {
            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    // now let's rename an import in a chain
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol";

        contract a {
            function go() public {
                mr_c x = new mr_c();

                assert(x.test() == 102);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        import { c as mr_c } from "c.sol";
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "c.sol".to_string(),
        r#"
        contract c {
            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);
}

#[test]
fn circular_import() {
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "self.sol".to_string(),
        r#"
        import { foo } from "self.sol";

        enum foo { foo1, foo2 }

        contract c {
            foo public f1;
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("self.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol";

        enum enum_a { f1, f2 }

        contract a {
            function go() public {
                b x = new b();

                assert(x.test() == 102);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        import "a.sol";
        contract b {
            function test() public returns (uint32) {
                return 102;
            }

            function test2() public returns (enum_a) {
                return enum_a.f1;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);
}

#[test]
fn import_symbol() {
    // import struct via import symbol
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol" as foo;

        contract a {
            function go(foo.b_struct x) public returns (uint32) {
                return x.f1;
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        struct b_struct {
            uint32 f1;
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    // import contract via import symbol
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol" as foo;

        contract a {
            function go() public returns (uint32) {
                foo.b x = new foo.b();

                return x.test();
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        contract b {
            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    // import enum in contract via import symbol
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol" as foo;

        contract a {
            function go(foo.b.c x) public {
                assert(x == foo.b.c.c2);
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        contract b {
            enum c { c1, c2 }

            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    // import struct in contract via import symbol chain
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol" as foo;

        contract a {
            function go(foo.bar.c.k x) public returns (int32) {
                return x.f1;
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        import "c.sol" as bar;
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "c.sol".to_string(),
        r#"
        contract c {
            struct k {
                int32 f1;
            }

            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);
}

#[test]
fn enum_import_chain() {
    // import struct in contract via import symbol chain
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        import "b.sol" as foo;

        contract a {
            function go(foo.c_import.d_import.d.enum_d x) public returns (bool) {
                return foo.c_import.d_import.d.enum_d.d2 == x;
            }
        }
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
        import "c.sol" as c_import;
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "c.sol".to_string(),
        r#"
        import "d.sol" as d_import;
        "#
        .to_string(),
    );

    cache.set_file_contents(
        "d.sol".to_string(),
        r#"
        contract d {
            enum enum_d { d1, d2, d3 }
        }
        "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    no_errors(ns.diagnostics);

    // now with error
    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
            import "b.sol" as foo;
    
            contract a {
                function go(foo.c_import.d_import.d.enum_d x) public returns (bool) {
                    return foo.c_import.d_import.d.enum_d.d4 == x;
                }
            }
            "#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol".to_string(),
        r#"
            import "c.sol" as c_import;
            "#
        .to_string(),
    );

    cache.set_file_contents(
        "c.sol".to_string(),
        r#"
            import "d.sol" as d_import;
            "#
        .to_string(),
    );

    cache.set_file_contents(
        "d.sol".to_string(),
        r#"
            contract d {
                enum enum_d { d1, d2, d3 }
            }
            "#
        .to_string(),
    );

    let ns = solang::parse_and_resolve("a.sol", &mut cache, Target::Substrate);

    assert_eq!(
        first_error(ns.diagnostics),
        "enum d.enum_d does not have value d4"
    );
}
