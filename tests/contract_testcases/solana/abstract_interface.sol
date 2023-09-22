abstract contract A {
	function v(int) public virtual;
}
contract C {
	function t(address id) public {
		A.v{program_id: id}(1);
	}
}

// ---- Expect: diagnostics ----
