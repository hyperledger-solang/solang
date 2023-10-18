contract c {
    function f() public pure returns (uint) {
	return 5e-2 + 1.95;
    }
    function g() public pure returns (uint) {
        return 200e-2 wei;
    }
    function h() public pure returns (uint) {
	return 5e-2 + 1.96;
    }
}

// ---- Expect: diagnostics ----
// warning: 6:16-26: ethereum currency unit used while targeting Solana
// error: 9:9-20: conversion to uint256 from rational not allowed
