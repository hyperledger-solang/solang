
        contract foo {
            function b32() public pure returns (bytes32 r) {
                r = bytes32(0xffee);
            }
        }
// ----
// error (105-120): number of 2 bytes cannot be converted to type 'bytes32'
