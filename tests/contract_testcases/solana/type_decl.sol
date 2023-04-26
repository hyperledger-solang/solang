
type Addr is address payable;

contract x {
	type Binary is bool;

	function f(Addr, Binary) public {}
}

// ----
// warning (69-100): function can be declared 'pure'
