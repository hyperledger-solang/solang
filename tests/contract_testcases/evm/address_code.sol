contract UpgradeableProxy {
    function _setImplementation(
        address newImplementation
    ) public pure returns (uint) {
        return newImplementation.code;
    }
}

// ---- Expect: diagnostics ----
// error: 5:9-38: conversion from uint8 slice to uint256 not possible
