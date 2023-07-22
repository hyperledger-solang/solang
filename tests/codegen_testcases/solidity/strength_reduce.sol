// RUN: --target polkadot --emit cfg
contract test {
/******************/
/* Multiply tests */
/******************/

// BEGIN-CHECK: test::function::f1
    function f1()  pure public {
        for (uint i = 0; i < 10; i++) {
            // this multiply can be done with a 64 bit instruction
            print("i:{}".format(i * 100));
        }
// CHECK: zext uint256 ((trunc uint64 %i) * uint64 100)
    }

// BEGIN-CHECK: test::function::f2
    function f2(bool x) pure public {
        uint i = 0;

        for (;;) {
            print("i:{}".format(i * 100));
            i += 1;
            if (x)
                break;
        }
// CHECK: %i * uint256 100
    }


// BEGIN-CHECK: test::function::f3
    function f3(bool x) pure public {
        uint i = 0;

        for (;;) {
            print("i:{}".format((i & 255) * 100));
            i += 1;
            if (x)
                break;
        }
// CHECK: (zext uint256 ((trunc uint64 (%i & uint256 255)) * uint64 100))
    }

// BEGIN-CHECK: test::function::f4
    function f4() pure public {
        for (uint i = 0; i < 10; i++) {
            unchecked {
                // this multiply can be done with a 64 bit instruction
                print("i:{}".format(i * 32768));
            }
        }
// CHECK: (%i << uint256 15)
        for (uint i = 0; i < 10; i++) {
            // Do not disable overflow checks
            print("i:{}".format(i * 32768));
        }
// NOT CHECK: (%i << uint256 15)
    }

// BEGIN-CHECK: test::function::f5
    function f5() pure public {
        for (int i = -50; i < -10; i++) {
            // this multiply can be done with a 64 bit instruction
            print("i:{}".format(i * 32769));
        }
// CHECK: sext int256 ((trunc int64 %i) * int64 32769)
    }

/******************/
/* Division tests */
/******************/

// BEGIN-CHECK: test::function::f6
    function f6() pure public {
        for (uint i = 1E9; i < 1E9+10; i++) {
            print("i:{}".format(i / 32768));
        }
// CHECK: (%i >> uint256 15)
    }

// BEGIN-CHECK: test::function::f7
    function f7(uint64 arg1) pure public {
        // we're upcasting to 256 bits, but known bits will track this
        uint i = arg1;
        print("i:{}".format(i / 1e6));
// CHECK: (zext uint256 (unsigned divide (trunc uint64 %i) / uint64 1000000)
    }

// BEGIN-CHECK: test::function::f8
    function f8() pure public {
        // too many values to track; (101 values)
        for (uint i = 1e9; i < 1e9+101; i++) {
            print("i:{}".format(i / 1e6));
        }
// CHECK: (unsigned divide %i / uint256 1000000)
    }


/****************/
/* Modulo tests */
/****************/

// BEGIN-CHECK: test::function::f9
    function f9() pure public {
        // too many values to track; (101 values)
        for (uint i = 1e9; i < 1e9+101; i++) {
            print("i:{}".format(i % 0x1_0000_0000));
        }
// CHECK: (%i & uint256 4294967295)
    }

// BEGIN-CHECK: test::function::f10
    function f10() pure public {
        for (int i = 30; i >= 0; i--) {
            print("i:{}".format(i % 0x1_0000_0001));
        }
// CHECK: sext int256 (signed modulo (trunc int64 %i) % int64 4294967297)
    }

// BEGIN-CHECK: test::function::f11
    function f11() pure public {
        for (int i = 0; i != 102; i++) {
            print("i:{}".format(i % 0x1_0000_0001));
        }
                // CHECK: (signed modulo %i % int256 4294967297)
    }
}
