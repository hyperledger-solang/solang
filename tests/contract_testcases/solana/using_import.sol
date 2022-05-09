import "simple.sol" as simpels;

function dec(simpels.S s) pure { s.f1 -= 1; }
using {dec} for simpels.S;

contract c {
	function test(simpels.S s) public {
		s.inc();
		s.dec();
	}
}
