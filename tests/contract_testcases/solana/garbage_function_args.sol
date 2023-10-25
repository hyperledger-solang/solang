@program_id("A8A3VYtDN69E72gceahcfVjLbf7m3c1u2RDwnbWgfRAk")
contract c {
	function g(address) public {
		require(rubbish, "rubbish n'existe pas!");
	}
	function g(bytes x) public {
		x.readUint8(meh);
	}
	function g() public {
		g({x: foo>1});
	}
	function g(int) external {
		this.g(oo);
	}
	function g(bool) external {
		this.g({x: foo>1});
	}
}

// ---- Expect: diagnostics ----
// error: 4:11-18: 'rubbish' not found
// error: 7:15-18: 'meh' not found
// error: 10:9-12: 'foo' not found
// error: 13:10-12: 'oo' not found
// error: 16:14-17: 'foo' not found
