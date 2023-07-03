
        contract c {
            struct s {
                uint32 x;
            mapping(uint => address) data;
            }

            function test() public {
                s memory x;

                x.data[1] = address(1);
            }
        }
// ---- Expect: diagnostics ----
// error: 9:17-18: mapping only allowed in storage
