
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int storage));
            }
        }
// ----
// error (123-124): unrecognised token ')', expected "case", "default", "leave", "revert", "switch", identifier
