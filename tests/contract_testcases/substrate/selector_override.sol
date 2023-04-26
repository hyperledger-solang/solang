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

// ----
// error (21-44): function 'new' selector must be 4 bytes rather than 2 bytes
// error (64-78): overriding selector not permitted on modifier
// error (99-113): overriding selector not permitted on receive
// error (146-164): overriding selector not permitted on fallback
// error (190-213): overriding selector only permitted on 'public' or 'external' function, not 'internal'
// error (241-264): overriding selector only permitted on 'public' or 'external' function, not 'private'
