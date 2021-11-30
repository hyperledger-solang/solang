use crate::build_solidity;

#[test]
fn global_constants() {
    let mut runtime = build_solidity(
        r##"
        int32 constant foo = 102 + 104;
        contract a {
            function test() public payable {
                assert(foo == 206);
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        string constant foo = "FOO";
        contract a {
            function test() public payable {
                assert(foo == "FOO");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());
}
