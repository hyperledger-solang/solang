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

// ----
// error (53-60): 'rubbish' not found
// error (135-138): 'meh' not found
// error (175-178): 'foo' not found
// error (222-224): 'oo' not found
// error (270-273): 'foo' not found
