
        contract main {
            function test() public {
                address x = address(0);

                x.staticcall(hex"1222");
            }
        }
// ---- Expect: diagnostics ----
// error: 6:19-29: method 'staticcall' does not exist
