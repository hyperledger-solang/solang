
        contract x {
            function b32() public pure returns (bytes32 r) {
                r = bytes32(0);
            }

            function b4() public pure returns (bytes4 r) {
                r = bytes4(0xcafedead);
            }

            function b3() public pure returns (bytes3 r) {
                r = bytes3(0x012233);
            }
        }
// ---- Expect: diagnostics ----
