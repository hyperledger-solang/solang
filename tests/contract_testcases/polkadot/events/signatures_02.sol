pragma solidity 0.5.4;

event foo(bool a, int b);

contract c {
	event foo(int b);
	event foo(int x);

	function f() public {
		// resolves to multiple events, but solc 0.5 permits this
		emit foo(true, 1);
	}
}

// ---- Expect: diagnostics ----
// warning: 6:8-11: event 'foo' has never been emitted
// warning: 7:8-11: event 'foo' has never been emitted
