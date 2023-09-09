// SPDX-License-Identifier: Apache-2.0

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
                assert(abi.encodePacked(a) == hex"01fd");
                uint32 b = 0xaabbccdd;
                assert(abi.encodePacked(true, b, false) == hex"01ddccbbaa00");
            }

            function test2() public {
                string b = "foobar";
                bytes c = abi.encodePacked(b);
                assert(abi.encodePacked(b) == "foobar");

                assert(abi.encodePacked("foobar") == "foobar");
                assert(abi.encodePacked("foo", "bar") == "foobar");
            }

            function test3() public {
                s x = s({ f1: 511, f2: 0xf7, f3: "testie", f4: [ 4, 5 ] });

                assert(abi.encodePacked(x) == hex"ff010000f774657374696504000500");
            }

            function test4() public {
                uint8[] vec = new uint8[](2);
                vec[0] = 0xca;
                vec[1] = 0xfe;
                assert(abi.encodePacked(vec) == hex"cafe");
            }
        }"#,
    );

    let account = vm.initialize_data_account();

    vm.function("new")
        .accounts(vec![("dataAccount", account)])
        .call();

    vm.function("test").call();

    vm.function("test2").call();

    vm.function("test3").call();

    vm.function("test4").call();
}

#[test]
fn inherited() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public {
            }
        }

        contract bar is foo { }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    vm.function("test").call();

    let mut vm = build_solidity(
        r#"
            contract foo {
                int public test;
            }

            contract bar is foo { }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    vm.function("test")
        .accounts(vec![("dataAccount", data_account)])
        .call();
}
