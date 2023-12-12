function f(int, int) pure {}
function f(int, bool) pure {}
contract C {
	function f(int, int) public pure {}
	function f(bool) public pure {}
	function f(bool, int) public pure {}
	function g() public pure {
		f(1, 2);
	}
}

// ---- Expect: diagnostics ----
// warning: 4:11-12: f is already defined as a function
// 	note 1:10-11: location of previous definition
// 	note 2:1-27: location of previous definition
