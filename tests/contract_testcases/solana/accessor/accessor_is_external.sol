contract C {
	int constant public c = 102;

	function f() public {
		int x = c();
	}
}
// ---- Expect: diagnostics ----
// error: 5:11-14: accessor function cannot be called via an internal function call
// 	note 2:22-23: declaration of 'c'
