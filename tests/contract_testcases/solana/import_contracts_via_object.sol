import "./simple.sol" as IMP;

contract C is IMP.A {
	using IMP.L for *;
	constructor() IMP.A() {
		revert IMP.E();
	}
	function foo() public {
		revert IMP.E({foo: 1});
	}
}

// ---- Expect: diagnostics ----
// error: 6:3-17: revert with custom errors not supported on Solana
// error: 6:10-15: error 'E' has 1 fields, 0 provided
// 	note 3:7-8: definition of 'E'
// error: 9:3-25: revert with custom errors not supported on Solana
