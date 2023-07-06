contract bar {
	uint128 state;
	function test(uint128 v) public returns (bool) {
		return state > v;
       }
}
// ---- Expect: diagnostics ----
// warning: 3:2-48: function can be declared 'view'
