contract base {
	function func() selector=hex"abcdef01" public virtual {}
}

contract child is base {
	function func() selector=hex"abcdef02" public override {}
}

contract child2 is base {
	function func() public override {}
}

contract base2 {
	function func() public virtual {}
}

contract child3 is base2 {
	function func() selector=hex"abcdef02" public override {}
}


