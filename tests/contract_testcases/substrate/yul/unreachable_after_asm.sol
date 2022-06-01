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
