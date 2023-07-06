enum CallFlag {
    FORWARD_INPUT,
    CLONE_INPUT,
    TAIL_CALL,
    ALLOW_REENTRY
}

function conv_flag(CallFlag[] _flags) pure returns (uint32 flags) {
    for (uint n = 0; n < _flags.length; n++) {
        flags |= (2 ** uint32(_flags[n]));
    }
}

contract Caller {
    function echo(
        address _address,
        bytes4 _selector,
        uint32 _x,
        CallFlag[] _flags
    ) public payable returns (uint32 ret) {
        bytes input = abi.encode(_selector, _x);
        (bool ok, bytes raw) = _address.call{value: conv_flag(_flags)}(input);
        require(ok);
        ret = abi.decode(raw, (uint32));
    }
}

// ---- Expect: diagnostics ----
