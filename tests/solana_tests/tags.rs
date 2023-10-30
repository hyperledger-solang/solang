// SPDX-License-Identifier: Apache-2.0

use crate::parse_and_resolve;
use solang::Target;
use solang_parser::pt;

#[test]
fn contract() {
    let ns = parse_and_resolve(
        r#"
        /// So
        /// @title Test
        /// @notice Hello,
        /// @notice World
        /// @author Mr Foo
        /// @dev this is
        ///  a contract
        /// @custom:meh words for
        /// @custom:meh custom tag
        /// @custom: custom tag
        @program_id("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh")
        contract test {
            /// @dev construct this
            @selector([1,2,3,4,5,6,7,8])
            constructor() {}
        }"#,
        Target::Solana,
    );

    assert_eq!(
        ns.diagnostics.first_error(),
        "custom tag '@custom:' is missing a name"
    );

    assert_eq!(ns.contracts[0].tags[0].tag, "notice");
    assert_eq!(ns.contracts[0].tags[0].value, "So Hello, World");

    assert_eq!(ns.contracts[0].tags[1].tag, "title");
    assert_eq!(ns.contracts[0].tags[1].value, "Test");

    assert_eq!(ns.contracts[0].tags[2].tag, "author");
    assert_eq!(ns.contracts[0].tags[2].value, "Mr Foo");

    assert_eq!(ns.contracts[0].tags[3].tag, "dev");
    assert_eq!(ns.contracts[0].tags[3].value, "this is\na contract");

    assert_eq!(ns.contracts[0].tags[4].tag, "custom:meh");
    assert_eq!(ns.contracts[0].tags[4].value, "words for custom tag");

    let constructor = ns
        .functions
        .iter()
        .find(|func| func.ty == pt::FunctionTy::Constructor)
        .unwrap();

    assert_eq!(constructor.tags[0].tag, "dev");
    assert_eq!(constructor.tags[0].value, "construct this");

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
        Target::Solana,
    );

    assert_eq!(ns.contracts[0].tags[0].tag, "notice");
    assert_eq!(ns.contracts[0].tags[0].value, "So Hello, World");

    assert_eq!(ns.contracts[0].tags[1].tag, "title");
    assert_eq!(ns.contracts[0].tags[1].value, "Test");

    assert_eq!(ns.contracts[0].tags[2].tag, "author");
    assert_eq!(ns.contracts[0].tags[2].value, "Mr Foo");

    assert_eq!(ns.contracts[0].tags[3].tag, "dev");
    assert_eq!(ns.contracts[0].tags[3].value, "this is\na contract");
}

#[test]
fn struct_tag() {
    let ns = parse_and_resolve(
        r#"
        /// @param f1 asdad
        /// @param f2 bar
        struct x {
            uint32 f1;
            uint32 f2;
        }"#,
        Target::Solana,
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
        /// @param f1 asdad
        /// @param f2 bar
        event x (
            uint32 f1,
            uint32 f2
        );"#,
        Target::Solana,
    );

    //Event never emitted generates a warning
    assert_eq!(ns.diagnostics.len(), 1);

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
        Target::Solana,
    );

    //Event never emitted generates a warning
    assert_eq!(ns.diagnostics.len(), 2);

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
        /**
         *  @dev bla bla bla
         * @author f2 bar */
        enum x { x1 }"#,
        Target::Solana,
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
        contract c is b {
            /// @param x sadad
            /// @return k is a boolean
            /// @inheritdoc b
            function foo(int x) public pure returns (int a, bool k) {}
        }

        contract b {}"#,
        Target::Solana,
    );

    assert_eq!(ns.diagnostics.len(), 5);

    let func = ns
        .functions
        .iter()
        .find(|func| func.id.name == "foo")
        .unwrap();

    assert_eq!(func.tags[0].tag, "param");
    assert_eq!(func.tags[0].value, "sadad");
    assert_eq!(func.tags[0].no, 0);
    assert_eq!(func.tags[1].tag, "return");
    assert_eq!(func.tags[1].value, "is a boolean");
    assert_eq!(func.tags[1].no, 1);
    assert_eq!(func.tags[2].tag, "inheritdoc");
    assert_eq!(func.tags[2].value, "b");
    assert_eq!(func.tags[2].no, 0);

    let ns = parse_and_resolve(
        r#"
        contract c is b {
            /// @return x sadad
            /// @param k is a boolean
            /// @custom:smtchecker abstract-function-nondet
            function foo(int x) public pure returns (int a, bool k) {}
        }

        contract b {}"#,
        Target::Solana,
    );

    assert_eq!(ns.diagnostics.len(), 4);

    assert_eq!(
        ns.diagnostics.first_error(),
        "function return value named 'x' not found"
    );

    assert_eq!(
        ns.diagnostics.first_warning().message,
        "'@param' used in stead of '@return' for 'k'"
    );

    let func = ns
        .functions
        .iter()
        .find(|func| func.id.name == "foo")
        .unwrap();

    assert_eq!(func.tags[0].tag, "return");
    assert_eq!(func.tags[0].value, "is a boolean");
    assert_eq!(func.tags[0].no, 1);
    assert_eq!(func.tags[1].tag, "custom:smtchecker");
    assert_eq!(func.tags[1].value, "abstract-function-nondet");
    assert_eq!(func.tags[1].no, 0);
}

#[test]
fn variables() {
    let ns = parse_and_resolve(
        r#"
        contract c is b {
            /// @notice so here we are
            /// @title i figured it out
            /// @inheritdoc b
            int y;
        }

        contract b {}"#,
        Target::Solana,
    );

    //Variable 'y' has never been used (one item error in diagnostic)
    assert_eq!(ns.diagnostics.len(), 3);

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
