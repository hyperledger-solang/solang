contract UpgradeableProxy {
    function _setImplementation(
        address newImplementation
    ) public view returns (bytes memory) {
        return newImplementation.code;
    }
}

// ---- Expect: diagnostics ----
