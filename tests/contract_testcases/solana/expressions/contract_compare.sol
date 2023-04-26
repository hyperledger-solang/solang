
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

// ----
// error (97-148): function 'cmp' overrides function in same contract
// 	note (15-66): previous definition of 'cmp'
// error (160-164): expression of type contract d not allowed
