
        contract main {
            function test() public {
                address x = address(0);

                (bool success, bytes bs) = x.call{gas: 5}(hex"1222");
            }
        }
// ----
// error (153-159): 'gas' not permitted for external calls or constructors on solana
