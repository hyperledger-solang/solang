
contract test3 {
	function foo(uint32 a) returns (uint32) {
		uint32 b = 50 - a;
		uint32 c;
		c = 100 * b;
		c += 5;
		return a * 1000 + c;
	}

	function bar(uint32 b, bool x) returns (uint32) {
		uint32 i = 1;
		if (x) {
			do {
				i += 10;
			}
			while (b-- > 0);
		} else {
			uint32 j;
			for (j=2; j<100; j++) {
				i *= 3;
			}
		}
		return i;
	}

	function baz(uint32 x) returns (uint32) {
		int l = 100;
		for (uint i = 0; i<100; i++) {
			x *= 7;

			if (x > l) {
				break;
			}

			x++;
		}

		return x;
	}
}
