error Unauthorized(bool);

contract VendingMachine {
    function withdraw() public pure {
        revert Unauthorized("foo");
    }
}

// ---- Expect: diagnostics ----
// error: 5:29-34: conversion from bytes3 to bool not possible
