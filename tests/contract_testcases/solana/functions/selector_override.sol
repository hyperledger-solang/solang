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

contract TooLarge {
    @selector([1, 2, 3, 256])
    function get_foo() pure public returns (int) {
        return 102;
    }

    @selector([0x05, 0x06, 0x07, 0xab8])
    function get_bar() pure public returns (int) {
        return 105;
    }
}
