
type Addr is address payable;

contract x {
	type Binary is bool;

	function f(Addr, Binary) public {}
}

// ---- Expect: diagnostics ----
// warning: 7:2-33: function can be declared 'pure'
