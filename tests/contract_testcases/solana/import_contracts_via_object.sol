import "simple.sol" as IMP;

contract C is IMP.A {
	using IMP.L for *;
	constructor() IMP.A() {
		revert IMP.E();
	}
	function foo() public {
		revert IMP.E({foo: 1});
	}
}

// ----
// error (98-112): revert with custom errors not supported on solana
// error (105-110): error 'E' has 1 fields, 0 provided
// 	note (33-34): definition of 'E'
// error (144-166): revert with custom errors not supported on solana
