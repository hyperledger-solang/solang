
contract test3 {
	function foo(uint32 a) returns (uint32) {
		uint32 b = 2;
		uint32 c;
		c = 100 * b;
		return a * 100 + c;
	}
}
