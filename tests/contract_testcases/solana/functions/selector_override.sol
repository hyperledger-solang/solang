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

// ---- Expect: diagnostics ----
// error: 2:2-25: overriding selector not permitted on constructor
// error: 4:2-16: overriding selector not permitted on modifier
// error: 6:2-16: overriding selector not permitted on receive
// error: 7:2-28: target Solana does not support receive() functions, see https://solang.readthedocs.io/en/latest/language/functions.html#fallback-and-receive-function
// error: 8:2-20: overriding selector not permitted on fallback
// error: 10:2-25: overriding selector only permitted on 'public' or 'external' function, not 'internal'
// error: 12:2-25: overriding selector only permitted on 'public' or 'external' function, not 'private'
// error: 17:25-28: value 256 does not fit into type uint8.
// error: 22:34-39: value 2744 does not fit into type uint8.
