contract c {
	// private functions cannot be payable and solc checks msg.value as
	// state read in them
	function foo() private view returns (uint) {
		return msg.value;
	}

	function bar() public returns (uint) {
		return foo();
	}
}

// ---- Expect: diagnostics ----
// warning: 8:2-38: function can be declared 'view'
