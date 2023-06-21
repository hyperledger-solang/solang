contract CallFlags {

    enum CallFlag { FORWARD_INPUT, CLONE_INPUT, TAIL_CALL, ALLOW_REENTRY }

    // Set all flags found in _flags to true.
    function bitflags(CallFlag[] _flags) internal pure returns (uint32 flags) {
        for (uint n = 0; n < _flags.length; n++) {
            flags |= (2 ** uint32(_flags[n]));
        }
    }

    // Call the contract at _address with the given _selector.
    // Specify any flag used for the contract call in _flags.
    function call_with_flags(
        address _address,
        bytes4 _selector,
        CallFlag[] _flags
    ) public returns (bytes ret) {
        uint32 call_flags = bitflags(_flags);
        (bool ok, ret) = _address.call{flags: call_flags}(_selector);
        require(ok);
    }
}

// ---- Expect: diagnostics ----