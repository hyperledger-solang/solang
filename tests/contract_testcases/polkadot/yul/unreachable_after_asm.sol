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
// warning: 2:5-14: storage variable 'b' has been assigned, but never read
// warning: 5:19-32: flag 'memory-safe' not supported
// warning: 5:34-39: flag 'meh' not supported
// warning: 10:9-12:10: unreachable statement
