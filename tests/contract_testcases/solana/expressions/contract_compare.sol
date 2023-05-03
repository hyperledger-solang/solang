
contract c {
	function cmp(d left, d right) public returns (bool) {
		return left < right;
	}

	function cmp(d left, e right) public returns (bool) {
		return left > right;
	}

}

contract d {}
contract e {}

// ---- Expect: diagnostics ----
// error: 7:2-53: function 'cmp' overrides function in same contract
// 	note 3:2-53: previous definition of 'cmp'
// error: 8:10-14: expression of type contract d not allowed
