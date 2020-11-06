
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
