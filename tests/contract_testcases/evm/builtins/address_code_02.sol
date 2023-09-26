contract UpgradeableProxy {
    function _setImplementation(
        address newImplementation
    ) public pure returns (bytes memory) {
        return newImplementation.code;
    }
}

// ---- Expect: diagnostics ----
// error: 5:16-38: function declared 'pure' but this expression reads from state
