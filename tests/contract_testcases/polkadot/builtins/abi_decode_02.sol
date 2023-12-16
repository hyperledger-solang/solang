
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int feh));
            }
        }
// ---- Expect: diagnostics ----
// error: 4:52-55: unexpected identifier 'feh'
