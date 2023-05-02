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
	function g(int) public {
		this.g(oo);
	}
	function g(bool) public {
		this.g({x: foo>1});
	}
}

// ---- Expect: diagnostics ----
// error: 3:11-18: 'rubbish' not found
// error: 6:15-18: 'meh' not found
// error: 9:9-12: 'foo' not found
// error: 12:10-12: 'oo' not found
// error: 15:14-17: 'foo' not found
