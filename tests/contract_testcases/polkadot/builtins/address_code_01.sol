contract UpgradeableProxy {
    function _setImplementation(
        address newImplementation
    ) public pure returns (bytes) {
        return newImplementation.code;
    }
}

// ---- Expect: diagnostics ----
// error: 5:16-33: 'address.code' is not supported on Polkadot
