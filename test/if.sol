
contract test3 {
	function foo(uint32 a) returns (uint32) {
		uint32 b = 50 - a;
		uint32 c;
		c = 100 * b;
		if (a == 1) {
			c += 5;
		} else {
			if (a == 2) {	
				c -= 5;
			} else if (a == 3) {
				c -= 10;
			} else {
				c -= 9;
			}
		}
		return a * 1000 + c;
	}
}
