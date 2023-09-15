error Unauthorized();

contract VendingMachine {
    function withdraw() public pure {
        revert Unauthorized();
    }
}

// ---- Expect: diagnostics ----
