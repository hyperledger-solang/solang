
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,mapping(uint[] => address)));
            }
        }
// ---- Expect: diagnostics ----
// error: 4:60-66: key of mapping cannot be array type
