
        contract foo {
            bytes32 public x = 0x00;
            bytes32 public y = 0x00_00_00;
            bytes3 public z1 = 0x10_00_00;
            // same length
            bytes3 public z2 = 0x00_00_00;
            // longer
            bytes3 public z3 = 0x00_00_00_00;

            function b32() public pure returns (bytes32 r) {
                r = bytes32(-1);
            }
        }
// ---- Expect: diagnostics ----
// error: 12:21-32: negative number cannot be converted to type 'bytes32'
