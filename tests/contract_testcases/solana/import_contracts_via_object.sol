import "simple.sol" as IMP;

contract C is IMP.A {
	using IMP.L for *;
	constructor() IMP.A() {
		revert IMP.E();
	}
}
