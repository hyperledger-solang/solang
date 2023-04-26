
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
// ----
// warning (54-55): function parameter 'l' has never been read
// warning (132-137): storage variable 'x' has never been used
// warning (201-202): declaration of 'x' shadows state variable
// 	note (132-137): previous declaration of state variable
// warning (201-202): local variable 'x' has been assigned, but never read
// warning (271-272): conversion truncates uint256 to uint128, as value is type uint128 on target substrate
