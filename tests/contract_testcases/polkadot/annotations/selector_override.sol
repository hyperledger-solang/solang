contract selector {
	@selector([0xab, 0xcd])
	constructor() {}
	@selector([x])
	modifier m() {_;}
	@selector([1])
	receive() payable external {}
	@selector([0xabc])
	fallback() external {}
	@selector([0xab, 0xdd])
	function i() internal {}
	@selector([0xab, 0xdd])
	function p() private {}
}

// ---- Expect: diagnostics ----
// error: 2:2-25: function 'new' selector must be 4 bytes rather than 2 bytes
// error: 4:2-16: overriding selector not permitted on modifier
// error: 6:2-16: overriding selector not permitted on receive
// error: 8:2-20: overriding selector not permitted on fallback
// error: 10:2-25: overriding selector only permitted on 'public' or 'external' function, not 'internal'
// error: 12:2-25: overriding selector only permitted on 'public' or 'external' function, not 'private'
