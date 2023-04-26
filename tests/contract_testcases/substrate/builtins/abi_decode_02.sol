
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", (int feh));
            }
        }
// ----
// error (116-119): unexpected identifier 'feh' in type
