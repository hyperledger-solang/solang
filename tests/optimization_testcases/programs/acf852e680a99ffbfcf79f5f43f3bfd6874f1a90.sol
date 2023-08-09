contract RH {
    function calc(
        uint256[] memory separators,
        int256[] memory params
    ) public pure returns (int256[4] memory) {
        int256 stopLimit = params[separators[4]];
        int256 contractedValueRatio = params[separators[6]];

        return [stopLimit, contractedValueRatio, 3, 4];
    }
}
