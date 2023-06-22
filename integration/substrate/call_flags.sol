contract CallFlags {
    uint8 roundtrips;

    // See https://github.com/paritytech/substrate/blob/5ea6d95309aaccfa399c5f72e5a14a4b7c6c4ca1/frame/contracts/src/wasm/runtime.rs#L373
    enum CallFlag { FORWARD_INPUT, CLONE_INPUT, TAIL_CALL, ALLOW_REENTRY }
    function bitflags(CallFlag[] _flags) internal pure returns (uint32 flags) {
        for (uint n = 0; n < _flags.length; n++) {
            flags |= (2 ** uint32(_flags[n]));
        }
    }

    // Reentrancy is required for reaching the `foo` function for itself.
    //
    // Cloning and forwarding should have the effect of calling this function again, regardless of what _address was passed.
    // Furthermore:
    // Cloning the input should work together with reentrancy.
    // Forwarding the input should fail due to reading the input more than once in the loop
    // Tail call should work with any combination of input forwarding.
    function echo(
        address _address,
        bytes4 _selector,
        uint32 _x,
        CallFlag[] _flags
    ) public payable returns(uint32 ret) {
        for (uint n = 0; n < 2; n++) {
            if (roundtrips > 1) {
                return _x;
            }
            roundtrips += 1;

            bytes input = abi.encode(_selector, _x);
            (bool ok, bytes raw) =  _address.call{flags: bitflags(_flags)}(input);
            require(ok);
            ret = abi.decode(raw, (uint32));

            roundtrips -= 1;
        }
    }

    @selector([0,0,0,0])
    function foo(uint32 x) public pure returns(uint32) {
        return x;
    }

    // Yields different result for tail calls
    function tail_call_it(
        address _address,
        bytes4 _selector,
        uint32 _x,
        CallFlag[] _flags
    ) public returns(uint32 ret) {
        bytes input = abi.encode(_selector, _x);
        (bool ok, bytes raw) =  _address.call{flags: bitflags(_flags)}(input);
        require(ok);
        ret = abi.decode(raw, (uint32));
        ret += 1;
    }

    // Does this.call() on this instead of address.call()
    function call_this(uint32 _x) public pure returns (uint32) {
        return this.foo{flags: bitflags([CallFlag.ALLOW_REENTRY])}(_x);
    }
}
