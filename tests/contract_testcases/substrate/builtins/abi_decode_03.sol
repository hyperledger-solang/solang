
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int,));
            }
        }
// ----
// error (116-116): missing type
