contract testTypes {
    uint256 b;
    function testAsm(uint128[] calldata vl) public {
        uint256 y = 0;
        assembly ("memory-safe", "meh") {
            switch vl.length
            case 1 {y := mul(b.slot, 2)}
            case 2 {y := shr(b.offset, 2)}
            default {
                y := 5
            }

            y := sub(y, 1)
            revert(y, 2)
        }

        if (vl[0] > 0) { 
            b = 5;
        }
    }
} 
