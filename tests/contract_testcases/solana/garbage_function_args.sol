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
