
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,));
            }
        }
// ---- Expect: diagnostics ----
// error: 4:52: stray comma
