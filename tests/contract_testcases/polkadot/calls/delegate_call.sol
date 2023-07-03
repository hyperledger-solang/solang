contract Delegate {
    function delegate(
        address callee,
        bytes input
    ) public returns(bytes result) {
        (bool ok, result) = callee.delegatecall{gas: 123}(input);
        require(ok);
    }
}

// ---- Expect: diagnostics ----
// warning: 6:29-65: 'gas' specified on 'delegatecall' will be ignored
