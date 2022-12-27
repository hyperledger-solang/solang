contract base {
	@selector([0xab, 0xcd, 0xef, 0x01])
	function func() public virtual {}
}

contract child is base {
	@selector([0xab, 0xcd, 0xef, 0x02])
	function func() public override {}
}

contract child2 is base {
	function func() public override {}
}

contract base2 {
	function func() public virtual {}
}

contract child3 is base2 {
	@selector([0xab, 0xcd, 0xef, 0x02])
	function func() public override {}
}