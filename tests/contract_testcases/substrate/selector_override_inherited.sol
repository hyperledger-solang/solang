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
// ---- Expect: diagnostics ----
// error: 7:2-37: selector of function 'func' different from base selector
// 	note 2:2-37: location of base function
// error: 12:2-33: selector of function 'func' must match base selector
// 	note 2:2-37: location of base function
// error: 20:2-37: base function needs same selector as selector of function 'func'
// 	note 16:2-32: location of base function
