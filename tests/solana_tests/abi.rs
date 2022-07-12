use crate::build_solidity;

#[test]
fn packed() {
    let mut vm = build_solidity(
        r#"
        struct s {
            int32 f1;
            uint8 f2;
            string f3;
            uint16[2] f4;
        }

        contract bar {
            function test() public {
                uint16 a = 0xfd01;
                assert(abi.encodePacked(a) == hex"fd01");
                uint32 b = 0xaabbccdd;
                assert(abi.encodePacked(true, b, false) == hex"01aabbccdd00");
            }

            function test2() public {
                string b = "foobar";
                assert(abi.encodePacked(b) == "foobar");

                assert(abi.encodePacked("foobar") == "foobar");
                assert(abi.encodePacked("foo", "bar") == "foobar");
            }

            function test3() public {
                s x = s({ f1: 511, f2: 0xf7, f3: "testie", f4: [ 4, 5 ] });

                assert(abi.encodePacked(x) == hex"000001fff774657374696500040005");
            }
        }"#,
    );

    vm.constructor("bar", &[]);

    vm.function("test", &[], &[], None);
    vm.function("test2", &[], &[], None);
    vm.function("test3", &[], &[], None);
}

#[test]
fn inherited() {
    let mut vm = build_solidity(
        r#"
        contract bar is foo { }

        contract foo {
            function test() public {
            }
        }"#,
    );

    vm.constructor("bar", &[]);

    vm.function("test", &[], &[], None);

    let mut vm = build_solidity(
        r#"
            contract bar is foo { }

            contract foo {
                int public test;
            }"#,
    );

    vm.constructor("bar", &[]);

    vm.function("test", &[], &[], None);
}
