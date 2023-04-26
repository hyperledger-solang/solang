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

// ----
// error (21-44): overriding selector not permitted on constructor
// error (64-78): overriding selector not permitted on modifier
// error (99-113): overriding selector not permitted on receive
// error (115-141): target solana does not support receive() functions, see https://solang.readthedocs.io/en/latest/language/functions.html#fallback-and-receive-function
// error (146-164): overriding selector not permitted on fallback
// error (190-213): overriding selector only permitted on 'public' or 'external' function, not 'internal'
// error (241-264): overriding selector only permitted on 'public' or 'external' function, not 'private'
// error (337-340): value 256 does not fit into type uint8.
// error (454-459): value 2744 does not fit into type uint8.
