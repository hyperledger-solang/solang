
        contract foo {
            function b32() public pure returns (bytes32 r) {
                r = bytes32(0xffee);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:21-36: number of 2 bytes cannot be converted to type 'bytes32'
