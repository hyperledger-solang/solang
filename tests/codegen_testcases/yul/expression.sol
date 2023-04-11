// RUN: --target solana --emit cfg -Onone --no-cse

uint128 constant global_cte = 5;
contract testing {

// BEGIN-CHECK: testing::testing::function::boolLiteral
    function boolLiteral() public pure {
        assembly {
            // CHECK: ty:uint256 %x = uint256(true)
            let x := true

            // CHECK: ty:bool %ss = (uint256 0 != uint256 5)
            let ss : bool := 5

            // CHECK: ty:uint256 %y = uint256(false)
            let y := false

            // CHECK: uint64(false)
            let r : u64 := false

            // CHECK: uint256 1
            let s := true :u128

            // CHECK: ty:uint32 %p = uint32 0
            let p :u32 := false :u64
        }
    }

// BEGIN-CHECK: testing::testing::function::numberLiteral
    function numberLiteral() public pure {
        assembly {
            // CHECK: ty:int32 %x = int32 90
            let x : s32 := 90

            // CHECK: ty:uint256 %y = uint256 128
            let y := 0x80

            // ty:uint256 %y = uint256 128
            let z := 0x80 : u8
        }
    }

// BEGIN-CHECK: testing::testing::function::stringLiteral
    function stringLiteral() public pure {
        assembly {
            // CHECK: ty:uint256 %x = uint256 427070744165
            let x := "coffe"

            // CHECK: ty:uint256 %y = uint256 831486
            let y := hex"0caffe"
        }
    }

    struct LocalTest {
        uint32 c;
    }

// BEGIN-CHECK: testing::testing::function::yulLocalVariable__uint256
    function yulLocalVariable(uint256 a) public pure {
        assembly {
            // CHECK: ty:uint256 %x = (overflowing uint256 2 + (arg #0))
            let x := add(2, a)

            // CHECK: ty:uint256 %y = (overflowing uint256 2 + (arg #0))
            let y := x
        }
    }

    uint32 constant contract_cte = 787;

    // BEGIN-CHECK: testing::testing::function::constantVariable
    function constantVariable() public pure {
        assembly {
            // CHECK: ty:uint256 %p1 = uint256 5
            let p1 := global_cte

            // CHECK: ty:uint256 %p2 = uint256 787
            let p2 := contract_cte
        }
    }

// BEGIN-CHECK: testing::testing::function::solidityLocalVariable
    function solidityLocalVariable() public pure {
        int32 a = 1;
        int[] vec;
        int[] memory mem_vec = vec;

        int[4] cte_vec = [int(1), 2, 3, 4];
        int[4] memory mem_cte_vec = [int(1), 2, 3, 4];

        string b = "abc";

        LocalTest struct_test = LocalTest({c: 2});
        LocalTest memory mem_struct_test = LocalTest({c: 2});
        assembly {
            // CHECK: ty:uint256 %k = uint256 1
            let k := a

            // CHECK: ty:uint256 %l = (zext uint256 uint64(%vec))
            let l := vec

            // CHECK: ty:uint256 %m = (zext uint256 uint64(%mem_vec))
            let m := mem_vec

            // CHECK: ty:uint256 %n = (zext uint256 uint64(%cte_vec))
            let n := cte_vec

            // CHECK: ty:uint256 %o = (zext uint256 uint64(%mem_cte_vec))
            let o := mem_cte_vec

            // CHECK: ty:uint256 %p = (zext uint256 uint64(%b))
            let p := b

            // CHECK: ty:uint256 %r = (zext uint256 uint64(%struct_test))
            let r := struct_test

            // CHECK: ty:uint256 %s = (zext uint256 uint64(%mem_struct_test))
            let s := mem_struct_test
        }
    }

    int[] storage_vec;
    int[] storage_vec_2;
    // BEGIN-CHECK: testing::testing::function::memberAccess__uint64:_uint64:3
    function memberAccess(uint64[] calldata vl, uint64[3] calldata vl_2) public view {
        int[] storage l_storage_vec = storage_vec_2;
        function () external fPtr = this.solidityLocalVariable;
        assembly {
            // CHECK: ty:uint256 %k = uint256 16
            let k := storage_vec.slot

            // CHECK: ty:uint256 %l = uint32 20
            let l := l_storage_vec.slot

            // CHECK: ty:uint256 %m = uint256 0
            let m := storage_vec.offset

            // CHECK: ty:uint256 %n = uint256 0
            let n := l_storage_vec.offset

            // CHECK: ty:uint256 %o = uint256((arg #0))
            let o := vl.offset

            // CHECK: ty:uint256 %p = (zext uint256 (builtin ArrayLength ((arg #0))))
            let p := vl.length

            // CHECK: ty:uint256 %q = uint256((load (struct %fPtr field 1)))
            let q := fPtr.address

            // CHECK: ty:uint256 %r = (zext uint256 bytes8((load (struct %fPtr field 0))))
            let r := fPtr.selector

            // CHECK: ty:uint256 %s = (zext uint256 uint64((arg #1)))
            let s := vl_2
        }
    }

// BEGIN-CHECK: testing::testing::function::exprFuncCall__uint32
    function exprFuncCall(uint32 a) public pure {
        assembly {
            function get(a) -> ret {
                ret := sub(a, 2)
            }

            function doSmth() {
                let y := 9
            }

            // CHECK: ty:int32 %k = (trunc int32 (overflowing (zext uint256 (arg #0)) + uint256 2))
            let k : s32 := add(a, 2)

            // CHECK: %ret.temp.58 = call testing::yul_function_0::get (sext uint256 (trunc int32 (overflowing (zext uint256 (arg #0)) + uint256 2)))
            let x := get(k)
            // CHECK: ty:uint256 %x = %ret.temp.58

            // CHECK: %ret1.temp.59, %ret2.temp.60 = call testing::yul_function_2::multipleReturns %x, (trunc int32 (overflowing (zext uint256 (arg #0)) + uint256 2))
            let l, m := multipleReturns(x, k)
            // CHECK: ty:uint256 %l = (zext uint256 %ret1.temp.59)
            // CHECK: ty:uint256 %m = (sext uint256 %ret2.temp.60)


            // CHECK: = call testing::yul_function_1::doSmth
            doSmth()

            function multipleReturns(a, v : s32) -> ret1 : u8, ret2 : s64 {
                ret1 := add(a, v)
                ret2 := mul(a, v)
            }
        }
    }
}
