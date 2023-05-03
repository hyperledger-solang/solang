contract Testing {

    uint16[2][4][] stor_arr;
    function getThis() public view returns (uint16) {
        uint16[2][4][] memory arr2 = stor_arr;
        return arr2[1][1][1];
    }
}
// ---- Expect: diagnostics ----
