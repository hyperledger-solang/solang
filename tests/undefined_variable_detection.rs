// SPDX-License-Identifier: Apache-2.0

use solang::codegen::{codegen, OptimizationLevel, Options};
use solang::file_resolver::FileResolver;
use solang::sema::ast::Diagnostic;
use solang::sema::ast::Namespace;
use solang::{parse_and_resolve, Target};
use std::ffi::OsStr;

fn parse_and_codegen(src: &'static str) -> Namespace {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());
    let mut ns = parse_and_resolve(
        OsStr::new("test.sol"),
        &mut cache,
        Target::default_polkadot(),
    );
    let opt = Options {
        dead_storage: false,
        constant_folding: false,
        strength_reduce: false,
        vector_to_slice: false,
        common_subexpression_elimination: false,
        opt_level: OptimizationLevel::Default,
        generate_debug_information: false,
        log_runtime_errors: false,
        log_prints: true,
        strict_soroban_types: false,
        #[cfg(feature = "wasm_opt")]
        wasm_opt: None,
        soroban_version: None,
    };

    codegen(&mut ns, &opt);

    ns
}

fn contains_error_message_and_notes(
    errors: &[&Diagnostic],
    message: &str,
    notes_no: usize,
) -> bool {
    for error in errors {
        if error.message == message {
            return error.notes.len() == notes_no;
        }
    }

    false
}

#[test]
fn used_before_being_defined() {
    let file = r#"
        contract Test {
        bytes byteArr;
        bytes32 baRR;

        function get() public  {
            string memory s = "Test";
            byteArr = bytes(s);
            uint16 a = 1;
            uint8 b;
            b = uint8(a);

            uint256 c;
            c = b;
            bytes32 b32;
            bytes memory char = bytes(bytes32(uint(a) * 2 ** (8 * b)));
            baRR = bytes32(c);
            bytes32 cdr = bytes32(char);
            assert(b32 == baRR);
            if(b32 != cdr) {

            }
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'b32' is undefined");
    assert_eq!(errors[0].notes.len(), 2);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );
    assert_eq!(
        errors[0].notes[1].message,
        "Variable read before being defined"
    );
}

#[test]
fn struct_as_ref() {
    let file = r#"
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
        }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'f' is undefined");
    assert_eq!(errors[0].notes.len(), 3);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );
    assert_eq!(
        errors[0].notes[1].message,
        "Variable read before being defined"
    );
    assert_eq!(
        errors[0].notes[2].message,
        "Variable read before being defined"
    );

    let file = r#"
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
                foo f = foo(false, 2);

                func(f);

                assert(f.x == true);
                assert(f.y == 64);
            }
        }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);
}

#[test]
fn while_loop() {
    let file = r#"
        contract testing {
        function test(int x) public pure returns (string) {
            string s;
            while(x > 0){
                s = "testing_string";
                x--;
            }

            return s;
        }
    }
    "#;
    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 's' is undefined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );

    let file = r#"
        contract testing {
        function test(int x) public pure returns (string) {
            string s;
            while(x > 0){
                s = "testing_string";
                x--;
            }

            if(x < 0) {
                s = "another_test";
            }

            return s;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 's' is undefined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );

    let file = r#"
        contract testing {
        function test(int x) public pure returns (string) {
            string s;
            while(x > 0){
                s = "testing_string";
                x--;
            }

            if(x < 0) {
                s = "another_test";
            } else {
                s = "should_work";
            }

            return s;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);
}

#[test]
fn for_loop() {
    let file = r#"
    contract testing {
        function test(int x) public pure returns (int) {
            int s;
            for(int i=0; i<x; i++) {
                s = 5;
            }

            int p;
            if(x < 0) {
                p = s + 2;
            }
            return p;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 2);
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'p' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 's' is undefined",
        1
    ));

    let file = r#"
    contract testing {
        function test(int x) public pure returns (int) {
            int s;

            for(int i=0; i<x; i++) {
                s = 5;
            }
            s=5;
            int p;
            if(x < 0) {
                p = s + 2;
            } else {
                p = 2;
                s = 2;
            }

            return p;
        }
    }
    "#;
    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);
}

#[test]
fn do_while_loop() {
    let file = r#"
    contract testing {
    struct other {
        int a;
    }
        function test(int x) public pure returns (int) {
            other o;
            do {
                x--;
                o = other(1);
            }while(x > 0);


            return o.a;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);
}

#[test]
fn if_else_condition() {
    let file = r#"
        contract testing {
        struct other {
            int a;
        }
        function test(int x) public pure returns (int) {
           other o;
           if(x > 0) {
               o = other(2);
           }

           return o.a;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'o' is undefined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );

    let file = r#"
        contract testing {
        struct other {
            int a;
        }
        function test(int x) public pure returns (int) {
           other o;
           if(x > 0) {
               x += 2;
           } else if(x < 0) {
               o = other(2);
           } else {
                x++;
           }

           return o.a;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'o' is undefined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );

    let file = r#"
        contract testing {
        struct other {
            int a;
        }
        function test(int x) public pure returns (int) {
           other o;
           if(x > 0) {
               o = other(2);
           } else {
               o = other(2);
           }

           return o.a;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);
}

#[test]
fn array() {
    let file = r#"
    contract test {

    function testing(int x) public pure returns (int) {
        int[] vec;

        return vec[0];
    }
    }
    "#;
    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);

    let file = r#"
    contract test {

    function testing(int x) public pure returns (int) {
        int[] vec;

        if(x > 0) {
            vec.push(2);
        }

        return vec[0];
    }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);
}

#[test]
fn contract_and_enum() {
    let file = r#"
contract other {
    int public a;

    function testing() public returns (int) {
        return 2;
    }
}


contract test {
    enum FreshJuiceSize{ SMALL, MEDIUM, LARGE }
    function testing(int x) public returns (int) {
        other o;
        FreshJuiceSize choice;
        if(x > 0 && o.testing() < 5) {
            o = new other();
        }

        assert(choice == FreshJuiceSize.LARGE);

        return o.a();
    }
}
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'o' is undefined",
        2
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'choice' is undefined",
        1
    ));
}

#[test]
fn basic_types() {
    let file = r#"
    contract test {

    function testing(int x) public returns (address, int, uint, bool, bytes, int) {
        address a;
        int i;
        uint u;
        bool b;
        bytes bt;
        int[5] vec;

        while(x > 0) {
            x--;
            a = address(this);
            i = -2;
            u = 2;
            b = true;
            bt = hex"1234";
            vec[0] = 2;
        }
        return (a, i, u, b, bt, vec[1]);
    }
}
    "#;
    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'bt' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'b' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'a' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'i' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'u' is undefined",
        1
    ));
}

#[test]
fn nested_branches() {
    let file = r#"

contract test {

    function testing(int x) public returns (int) {
        int i;

        while(x > 0) {
            int b;
           if(x > 5) {
               b = 2;
           }

           i = b;
        }
        int a;
        if(x < 5) {
            if(x < 2) {
                a = 2;
            } else {
                a = 1;
            }
        } else if(x < 4) {
            a = 5;
        }

        return i + a;
    }
}
    "#;
    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'b' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'a' is undefined",
        1
    ));
    assert!(contains_error_message_and_notes(
        &errors,
        "Variable 'i' is undefined",
        2
    ));
}

#[test]
fn try_catch() {
    let file = r#"
     contract AddNumbers { function add(uint256 a, uint256 b) external pure returns (uint256 c) {c = b;} }
     contract Example {
         AddNumbers addContract;
         event StringFailure(string stringFailure);
         event BytesFailure(bytes bytesFailure);
    
         function exampleFunction(uint256 _a, uint256 _b) public returns (bytes c) {
             bytes r;
             try addContract.add(_a, _b) returns (uint256 _value) {
                 r = hex"ABCD";
                 return r;
             } catch Error(string memory _err) {
                 r = hex"ABCD";
                 emit StringFailure(_err);
             } catch (bytes memory _err) {
                 emit BytesFailure(_err);
             }
    
             return r;
         }
    
     }
     "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'r' is undefined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );

    let file = r#"
    contract AddNumbers { function add(uint256 a, uint256 b) external pure returns (uint256 c) {c = b;} }
    contract Example {
        AddNumbers addContract;
        event StringFailure(string stringFailure);
        event BytesFailure(bytes bytesFailure);

        function exampleFunction(uint256 _a, uint256 _b) public returns (bytes c) {
            bytes r;
            try addContract.add(_a, _b) returns (uint256 _value) {
                r = hex"ABCD";
                return r;
            } catch Error(string memory _err) {
                r = hex"ABCD";
                emit StringFailure(_err);
            } catch (bytes memory _err) {
                r = hex"ABCD";
                emit BytesFailure(_err);
            }

            return r;
        }

    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 0);

    let file = r#"
    contract AddNumbers { function add(uint256 a, uint256 b) external pure returns (uint256 c) {c = b;} }
    contract Example {
        AddNumbers addContract;
        event StringFailure(string stringFailure);
        event BytesFailure(bytes bytesFailure);

        function exampleFunction(uint256 _a, uint256 _b) public returns (bytes c) {
            bytes r;
            try addContract.add(_a, _b) returns (uint256 _value) {
                return r;
            } catch Error(string memory _err) {
                r = hex"ABCD";
                emit StringFailure(_err);
            } catch (bytes memory _err) {
                emit BytesFailure(_err);
            }

            return r;
        }
    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'r' is undefined");
    assert_eq!(errors[0].notes.len(), 2);
    assert!(errors[0]
        .notes
        .iter()
        .all(|note| { note.message == "Variable read before being defined" }));

    let file = r#"
    contract AddNumbers { function add(uint256 a, uint256 b) external pure returns (uint256 c) {c = b;} }
    contract Example {
        AddNumbers addContract;
        event StringFailure(string stringFailure);
        event BytesFailure(bytes bytesFailure);

        function exampleFunction(uint256 _a, uint256 _b) public returns (bytes c) {
            bytes r;
            try addContract.add(_a, _b) returns (uint256 _value) {
                r = hex"ABCD";
                return r;
            } catch Error(string memory _err) {
                emit StringFailure(_err);
            } catch (bytes memory _err) {
                emit BytesFailure(_err);
            }

            return r;
        }

    }
    "#;

    let ns = parse_and_codegen(file);
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Variable 'r' is undefined");
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(
        errors[0].notes[0].message,
        "Variable read before being defined"
    );
}
