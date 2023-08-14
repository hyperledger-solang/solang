contract foo {
    function testMod() public pure returns (uint256 a, uint256 b) {
        assembly {
            let
                x
            := 115792089237316195423570985008687907853269984665640564039457584007913129639935
            let
                y
            := 115792089237316195423570985008687907853269984665640564039457584007913129639935

            a := mulmod(x, 2, 10)
            b := addmod(y, 2, 10)
        }

        return (a, b);
    }
}
