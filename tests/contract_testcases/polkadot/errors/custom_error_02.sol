error Unauthorized(bytes b);

contract VendingMachine {
    function withdraw() public pure {
        revert Unauthorized("foo");
    }
}

// ---- Expect: diagnostics ----
