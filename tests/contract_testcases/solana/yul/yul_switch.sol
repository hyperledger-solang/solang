contract testTypes {
    uint256 b;
    function testAsm(uint128[] calldata vl) public pure returns (uint256) {
        uint256 y = 0;
        assembly {
            switch vl.length
            case 1 {y := mul(b.slot, 2)}
            case 2 {y := shr(b.offset, 2)}
            default {
                y := 5
            }

            y := sub(y, 1)
        }

        return y;
    }
} 
// ---- Expect: diagnostics ----
