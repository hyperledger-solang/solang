use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[test]
fn abi_decode() {
    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", feh);
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "type ‘feh’ not found");

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int storage));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "storage modifier ‘storage’ not allowed"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int feh));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "unexpected identifier ‘feh’ in type"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "missing type");

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,mapping(uint[] => address)));
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "key of mapping cannot be array type"
    );

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                (int16 a, bool b) = abi.decode(hex"7f0001", (int16, bool));

                assert(a == 127);
                assert(b == true);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                uint8 a = abi.decode(hex"40", (uint8));

                assert(a == 64);
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}
