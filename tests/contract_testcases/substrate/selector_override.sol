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
