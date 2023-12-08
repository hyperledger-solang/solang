pragma solidity 0.4.12;

contract A {
	event E(int indexed a, bool indexed b) anonymous;
}

contract B is A {
	event E(int a, bool b);

	function test1() public {
		emit E(1, true);
	}

	function test2() public {
		emit E({a: 1, b: true});
	}
}

// ---- Expect: diagnostics ----
// warning: 11:3-18: emit can be resolved to multiple incompatible events. This is permitted in Solidity v0.5 and earlier, however it could indicate a bug.
// 	note 8:8-9: candidate event
// 	note 4:8-9: candidate event
// warning: 15:3-26: emit can be resolved to multiple incompatible events. This is permitted in Solidity v0.5 and earlier, however it could indicate a bug.
// 	note 8:8-9: candidate event
// 	note 4:8-9: candidate event
