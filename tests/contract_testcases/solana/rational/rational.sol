contract foo {
    function test() public returns (uint) {
        uint y = 0.1;
        return y;
    }

    // Ensure - after scientific notation is not lexed
    // https://github.com/hyperledger-solang/solang/issues/1065
    function test2() external {
        uint256 b = 500;
        uint256 a1 = 1e18-b;
        uint256 a2 = 1e18- b;
    }
}
// ---- Expect: diagnostics ----
// error: 3:18-21: conversion to uint256 from rational not allowed
