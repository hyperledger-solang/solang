import "./simple.sol" as simpels;

function dec(simpels.S s) pure { s.f1 -= 1; }
using {dec} for simpels.S;

contract c {
	function test(simpels.S s) public {
		s.inc();
		s.dec();
	}
}

// ---- Expect: diagnostics ----
// warning: 7:2-35: function can be declared 'pure'
// warning: 3:7-8: error 'E' has never been used
