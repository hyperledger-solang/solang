
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,mapping(uint[] => address)));
            }
        }