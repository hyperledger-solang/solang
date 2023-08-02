contract c {
    function getByte(uint256 bb) public pure returns (uint256) {
        uint256 ret = 0;
        assembly {
            ret := byte(5, bb)
        }
        return ret;
    }

    function divide(
        uint256 a,
        uint256 b
    ) public pure returns (uint256 ret1, uint256 ret2) {
        assembly {
            ret1 := div(a, b)
            ret2 := mod(a, b)
        }
    }

    function mods(
        uint256 a,
        uint256 b,
        uint256 c
    ) public pure returns (uint256 ret1, uint256 ret2) {
        assembly {
            ret1 := addmod(a, b, c)
            ret2 := mulmod(a, b, c)
        }
    }
}
