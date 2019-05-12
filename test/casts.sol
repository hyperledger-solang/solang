

contract foo {
	uint bar;

	function set_bar(uint32 b) public {
		bar = b;
	}

	function get_bar() public returns (uint32) {
		return uint32(bar);
	}
}

contract bar {
	enum X { Y1, Y2, Y3}
	X y;

	function set_x(uint32 b) public {
		y = X(b);
	}

	function get_x() public returns (uint32) {
		return uint32(y);
	}
}
