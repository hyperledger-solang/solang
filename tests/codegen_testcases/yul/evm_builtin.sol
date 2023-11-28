// RUN: --target evm --emit cfg -Onone --no-cse

contract Testing {

// BEGIN-CHECK: Testing::Testing::function::builtins__uint256
    function builtins(uint256 arg1) public view  {
        assembly {
            // CHECK: ty:uint256 %a = (zext uint256 (builtin Gasleft ()))
            let a := gas()

            // CHECK: (sext uint256 uint160((load (builtin GetAddress ()))))
            let b := address()

            // CHECK: ty:uint256 %c = uint256((builtin Balance (address((trunc uint160 (arg #0))))))
            let c := balance(arg1)

            // CHECK: ty:uint256 %d = uint256((builtin Balance ((load (builtin GetAddress ())))))
            let d := selfbalance()

            // CHECK: ty:uint256 %e = (sext uint256 uint160((builtin Sender ())))
            let e := caller()

            // CHECK: ty:uint256 %f = uint256((builtin Value ()))
            let f := callvalue()

            // CHECK: ty:uint256 %g = (zext uint256 (builtin Gasprice ()))
            let g := gasprice()

            // CHECK: ty:uint256 %h = (builtin BlockHash ((trunc uint64 (arg #0))))
            let h := blockhash(arg1)

            // CHECK: ty:uint256 %i = (sext uint256 uint160((builtin BlockCoinbase ())))
            let i := coinbase()

            // ty:uint256 %j = (zext uint256 (builtin Timestamp ()))
            let j := timestamp()

            // CHECK: ty:uint256 %k = (zext uint256 (builtin BlockNumber ()))
            let k := number()

            // CHECK: y:uint256 %l = (builtin BlockDifficulty ())
            let l := difficulty()

            // CHECK: ty:uint256 %m = (zext uint256 (builtin GasLimit ()))
            let m := gaslimit()

            // CHECK: ty:uint256 %n = (zext uint256 (builtin ExtCodeSize (address((trunc uint160 %b)))))
            let n := extcodesize(b)

            // CHECK: assert-failure
            invalid()
        }
    }

// BEGIN-CHECK: Testing::Testing::function::test_selfdestruct__uint256
    function test_selfdestruct(uint256 arg1) public {
        assembly {
            // CHECK: selfdestruct address payable((trunc uint160 (arg #0)))
            selfdestruct(arg1)
        }
    }
}
