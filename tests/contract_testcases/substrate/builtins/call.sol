
        contract main {
            function test() public {
                address x = address(0);

                x.delegatecall(hex"1222");
            }
        }
// ----
// error (121-133): method 'delegatecall' does not exist
