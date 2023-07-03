
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", feh);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:47-50: type 'feh' not found
