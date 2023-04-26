
        contract printer {
            function test() public {
                (int a) = abi.decode(hex"00", feh);
            }
        }
// ----
// error (111-114): type 'feh' not found
