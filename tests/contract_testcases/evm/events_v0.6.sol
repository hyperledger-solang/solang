pragma solidity ^0.6.5;

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
// error: 11:3-18: emit can be resolved to multiple events
// 	note 8:8-9: candidate event
// 	note 4:8-9: candidate event
// error: 15:3-26: emit can be resolved to multiple events
// 	note 8:8-9: candidate event
// 	note 4:8-9: candidate event
