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

// ----
// warning (147-157): ethereum currency unit used while targeting solana
// error (212-230): conversion to uint256 from rational not allowed
