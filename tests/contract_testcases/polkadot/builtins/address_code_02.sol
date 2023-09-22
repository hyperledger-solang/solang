contract UpgradeableProxy {
    function _setImplementation(address newImplementation) public view {
        assert(newImplementation.code.length != 0);
    }
}

// ---- Expect: diagnostics ----
// error: 3:16-33: 'address.code' is not supported on Polkadot
