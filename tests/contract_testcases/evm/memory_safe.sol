pragma solidity ^0.8.0;

library Issue1526 {
    function extSloads() public pure {
        assembly ("memory-safe", "foo", "memory-safe") {

        }
    }
}

// ---- Expect: diagnostics ----
// warning: 5:34-39: flag 'foo' not supported
// warning: 5:41-54: flag 'memory-safe' already specified
// 	note 5:19-32: previous location
