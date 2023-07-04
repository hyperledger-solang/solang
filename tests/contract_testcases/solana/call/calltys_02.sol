
        contract main {
            function test() public {
                address x = address(0);

                (bool success, bytes bs) = x.call{gas: 5}(hex"1222");
            }
        }
// ---- Expect: diagnostics ----
// error: 6:51-57: 'gas' not permitted for external calls or constructors on Solana
