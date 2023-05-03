struct error { int f1; }

function member(error e) pure returns (int) {
	return e.f1;
}

using {member} for error;

contract c {
	error public v1;

	function test(error e) internal pure returns (error) {
		e.member();
		return e;
	}
}

// ---- Expect: diagnostics ----
