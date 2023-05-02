
        contract main {
            function test() public {
                address x = address(0);

                x.delegatecall(hex"1222");
            }
        }
// ---- Expect: diagnostics ----
// error: 6:19-31: method 'delegatecall' does not exist
