use super::{first_error, parse_and_resolve};
use solang::Target;

#[test]
fn contract() {
    let ns = parse_and_resolve(
        r#"
        /// @barf
        contract test {}"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@barf’ is not valid for contract"
    );

    let ns = parse_and_resolve(
        r#"
        /// So
        /// @title Test
        /// @notice Hello,
        /// @notice World
        /// @author Mr Foo
        /// @dev this is
        ///  a contract
        contract test {}"#,
        Target::Substrate,
    );

    assert_eq!(ns.contracts[0].tags[0].tag, "notice");
    assert_eq!(ns.contracts[0].tags[0].value, "So Hello, World");

    assert_eq!(ns.contracts[0].tags[1].tag, "title");
    assert_eq!(ns.contracts[0].tags[1].value, "Test");

    assert_eq!(ns.contracts[0].tags[2].tag, "author");
    assert_eq!(ns.contracts[0].tags[2].value, "Mr Foo");

    assert_eq!(ns.contracts[0].tags[3].tag, "dev");
    assert_eq!(ns.contracts[0].tags[3].value, "this is a contract");

    let ns = parse_and_resolve(
        r#"
        /// So
        /// @title Test
        /// @notice Hello,
        /// @notice World
        /** @author Mr Foo
         * @dev this is
         * a contract
         */
        contract test {}"#,
        Target::Substrate,
    );

    assert_eq!(ns.contracts[0].tags[0].tag, "notice");
    assert_eq!(ns.contracts[0].tags[0].value, "So Hello, World");

    assert_eq!(ns.contracts[0].tags[1].tag, "title");
    assert_eq!(ns.contracts[0].tags[1].value, "Test");

    assert_eq!(ns.contracts[0].tags[2].tag, "author");
    assert_eq!(ns.contracts[0].tags[2].value, "Mr Foo");

    assert_eq!(ns.contracts[0].tags[3].tag, "dev");
    assert_eq!(ns.contracts[0].tags[3].value, "this is a contract");
}

#[test]
fn struct_tag() {
    let ns = parse_and_resolve(
        r#"
        /// @param
        struct x {
            uint32 f;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@param’ missing parameter name"
    );

    let ns = parse_and_resolve(
        r#"
        /// @param g
        struct x {
            uint32 f;
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "tag ‘@param’ no field ‘g’");

    let ns = parse_and_resolve(
        r#"
        /// @param f asdad
        /// @param f bar
        struct x {
            uint32 f;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate tag ‘@param’ for ‘f’"
    );

    let ns = parse_and_resolve(
        r#"
        /// @param f1 asdad
        /// @param f2 bar
        struct x {
            uint32 f1;
            uint32 f2;
        }"#,
        Target::Substrate,
    );

    assert_eq!(ns.diagnostics.len(), 0);

    assert_eq!(ns.structs[0].tags[0].tag, "param");
    assert_eq!(ns.structs[0].tags[0].value, "asdad");
    assert_eq!(ns.structs[0].tags[0].no, 0);

    assert_eq!(ns.structs[0].tags[1].tag, "param");
    assert_eq!(ns.structs[0].tags[1].value, "bar");
    assert_eq!(ns.structs[0].tags[1].no, 1);
}

#[test]
fn event_tag() {
    let ns = parse_and_resolve(
        r#"
        /// @param
        event x (
            uint32 f
        );"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@param’ missing parameter name"
    );

    let ns = parse_and_resolve(
        r#"
        /// @param g
        event x (
            uint32 f
        );"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "tag ‘@param’ no field ‘g’");

    let ns = parse_and_resolve(
        r#"
        /// @param f asdad
        /// @param f bar
        event x (
            uint32 f
        );"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate tag ‘@param’ for ‘f’"
    );

    let ns = parse_and_resolve(
        r#"
        /// @param f1 asdad
        /// @param f2 bar
        event x (
            uint32 f1,
            uint32 f2
        );"#,
        Target::Substrate,
    );

    assert_eq!(ns.diagnostics.len(), 0);

    assert_eq!(ns.events[0].tags[0].tag, "param");
    assert_eq!(ns.events[0].tags[0].value, "asdad");
    assert_eq!(ns.events[0].tags[0].no, 0);

    assert_eq!(ns.events[0].tags[1].tag, "param");
    assert_eq!(ns.events[0].tags[1].value, "bar");
    assert_eq!(ns.events[0].tags[1].no, 1);

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @title foo bar
            /// @author mr foo
            /// @param f1 asdad
            event x (
                uint32 f1,
                uint32 f2
            );
        }"#,
        Target::Substrate,
    );

    assert_eq!(ns.diagnostics.len(), 1);

    assert_eq!(ns.events[0].tags[0].tag, "title");
    assert_eq!(ns.events[0].tags[0].value, "foo bar");
    assert_eq!(ns.events[0].tags[0].no, 0);

    assert_eq!(ns.events[0].tags[1].tag, "author");
    assert_eq!(ns.events[0].tags[1].value, "mr foo");
    assert_eq!(ns.events[0].tags[1].no, 0);

    assert_eq!(ns.events[0].tags[2].tag, "param");
    assert_eq!(ns.events[0].tags[2].value, "asdad");
    assert_eq!(ns.events[0].tags[2].no, 0);
}

#[test]
fn enum_tag() {
    let ns = parse_and_resolve(
        r#"
        /// @param
        enum x {
            foo1
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@param’ is not valid for enum"
    );

    let ns = parse_and_resolve(
        r#"
        /**
         *  @dev bla bla bla
         * @author f2 bar */
        enum x { x1 }"#,
        Target::Substrate,
    );

    assert_eq!(ns.diagnostics.len(), 0);

    assert_eq!(ns.enums[0].tags[0].tag, "dev");
    assert_eq!(ns.enums[0].tags[0].value, "bla bla bla");
    assert_eq!(ns.enums[0].tags[0].no, 0);

    assert_eq!(ns.enums[0].tags[1].tag, "author");
    assert_eq!(ns.enums[0].tags[1].value, "f2 bar");
    assert_eq!(ns.enums[0].tags[1].no, 0);
}

#[test]
fn functions() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @param
            function foo() public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@param’ missing parameter name"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @param f
            /// @param g
            function foo(int f) public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "tag ‘@param’ no field ‘g’");

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @param f
            /**
             @param f asda
             */
            function foo(int f) public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate tag ‘@param’ for ‘f’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @return so here we are
            function foo() public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@return’ for function with no return values"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @return so here we are
            function foo() public returns (int a, bool) {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "tag ‘@return’ no field ‘so’");

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @return
            function foo() public returns (int a, bool b) {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@return’ missing parameter name"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @return a asda
            /// @return a barf
            function foo() public returns (int a, bool b) {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate tag ‘@return’ for ‘a’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @inheritdoc
            function foo() public returns (int a, bool b) {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "missing contract for tag ‘@inheritdoc’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @inheritdoc b
            function foo() public returns (int a, bool b) {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "base contract ‘b’ not found in tag ‘@inheritdoc’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c is b {
            /// @param x sadad
            /// @return k is a boolean
            /// @inheritdoc b
            function foo(int x) public pure returns (int a, bool k) {}
        }

        contract b {}"#,
        Target::Substrate,
    );

    assert_eq!(ns.diagnostics.len(), 2);

    assert_eq!(ns.contracts[0].functions[0].tags[0].tag, "param");
    assert_eq!(ns.contracts[0].functions[0].tags[0].value, "sadad");
    assert_eq!(ns.contracts[0].functions[0].tags[0].no, 0);

    assert_eq!(ns.contracts[0].functions[0].tags[1].tag, "return");
    assert_eq!(ns.contracts[0].functions[0].tags[1].value, "is a boolean");
    assert_eq!(ns.contracts[0].functions[0].tags[1].no, 1);

    assert_eq!(ns.contracts[0].functions[0].tags[2].tag, "inheritdoc");
    assert_eq!(ns.contracts[0].functions[0].tags[2].value, "b");
    assert_eq!(ns.contracts[0].functions[0].tags[2].no, 0);
}

#[test]
fn variables() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            /// @param
            int x;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "tag ‘@param’ is not valid for state variable"
    );

    let ns = parse_and_resolve(
        r#"
        contract c is b {
            /// @notice so here we are
            /// @title i figured it out
            /// @inheritdoc b
            int y;
        }

        contract b {}"#,
        Target::Substrate,
    );

    assert_eq!(ns.diagnostics.len(), 2);

    assert_eq!(ns.contracts[0].variables[0].tags[0].tag, "notice");
    assert_eq!(ns.contracts[0].variables[0].tags[0].value, "so here we are");
    assert_eq!(ns.contracts[0].variables[0].tags[0].no, 0);

    assert_eq!(ns.contracts[0].variables[0].tags[1].tag, "title");
    assert_eq!(
        ns.contracts[0].variables[0].tags[1].value,
        "i figured it out"
    );
    assert_eq!(ns.contracts[0].variables[0].tags[1].no, 0);

    assert_eq!(ns.contracts[0].variables[0].tags[2].tag, "inheritdoc");
    assert_eq!(ns.contracts[0].variables[0].tags[2].value, "b");
    assert_eq!(ns.contracts[0].variables[0].tags[2].no, 0);
}
