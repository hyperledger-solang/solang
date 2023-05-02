contract testTypes {
    uint256 b;
    function testAsm(uint128[] calldata vl) public {
        uint256 y = 0;
        assembly ("memory-safe", "meh") {
            y := sub(y, 1)
            invalid()
        }

        if (vl[0] > 0) { 
            b = 5;
        }
    }
} 

// ---- Expect: diagnostics ----
// error: 5:19-32: flag 'memory-safe' not supported
// error: 5:34-39: flag 'meh' not supported
// error: 10:9-12:10: unreachable statement
