error Unauthorized(bool);

contract VendingMachine {
    function withdraw() public pure {
        revert Unauthorized();
    }
}

// ---- Expect: diagnostics ----
// error: 5:16-28: error 'Unauthorized' has 1 fields, 0 provided
// 	note 1:7-19: definition of 'Unauthorized'
