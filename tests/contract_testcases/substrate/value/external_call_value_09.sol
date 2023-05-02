
        contract a {
            function test(int32 l) public payable {
            }
        }

        contract b {
            int x;

            function test() public {
                uint256 x = 500;
                a f = new a();
                f.test{value: x}({l: 501});
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:33-34: function parameter 'l' has never been read
// warning: 8:13-18: storage variable 'x' has never been used
// warning: 11:25-26: declaration of 'x' shadows state variable
// 	note 8:13-18: previous declaration of state variable
// warning: 11:25-26: local variable 'x' has been assigned, but never read
// warning: 13:31-32: conversion truncates uint256 to uint128, as value is type uint128 on target substrate
