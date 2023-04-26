
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,mapping(uint[] => address)));
            }
        }
// ----
// error (124-130): key of mapping cannot be array type
