contract UpgradeableProxy {
    function _setImplementation(
        address newImplementation
    ) public pure returns (uint) {
        return newImplementation.code;
    }
}

// ---- Expect: diagnostics ----
// error: 5:16-38: conversion from bytes to uint256 not possible
