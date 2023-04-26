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
// ----
// error (117-152): selector of function 'func' different from base selector
// 	note (17-52): location of base function
// error (219-250): selector of function 'func' must match base selector
// 	note (17-52): location of base function
// error (340-375): base function needs same selector as selector of function 'func'
// 	note (275-305): location of base function
