
        contract main {
            function test() public {
                address x = address(0);

                x.staticcall(hex"1222");
            }
        }
// ----
// error (121-131): method 'staticcall' does not exist
