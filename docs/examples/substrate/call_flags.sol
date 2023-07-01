library CallFlags {
    uint32 constant FORWARD_INPUT = 1;
    uint32 constant CLONE_INPUT = 2;
    uint32 constant TAIL_CALL = 4;
    uint32 constant ALLOW_REENTRY = 8;
}

contract Reentrant {
    function reentrant_call(
        address _address,
        bytes4 selector
    ) public returns (bytes ret) {
        (bool ok, ret) = _address.call{flags: CallFlags.ALLOW_REENTRY}(selector);
        require(ok);
    }
}
