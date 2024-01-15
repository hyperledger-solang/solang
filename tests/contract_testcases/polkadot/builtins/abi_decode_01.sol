
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int storage));
            }
        }

// ---- Expect: diagnostics ----
// error: 4:59-60: unrecognised token ')', expected "case", "default", "fallback", "leave", "receive", "revert", "switch", identifier
