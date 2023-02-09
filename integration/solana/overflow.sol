contract overflow {
	function addu32(uint32 a, uint32 b) public pure returns (uint32 c) {
		c = a + b;
	}

	function subu32(uint32 a, uint32 b) public pure returns (uint32 c) {
		c = a - b;
	}

	function mulu32(uint32 a, uint32 b) public pure returns (uint32 c) {
		c = a * b;
	}

	function powu32(uint32 a, uint32 b) public pure returns (uint32 c) {
		c = a ** b;
	}
}
