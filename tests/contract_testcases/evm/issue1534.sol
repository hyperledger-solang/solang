// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Overloaded {
    // Version 1
    function f(uint n) internal pure returns (uint) {
        return n;
    }

    // Version 3
    function f(int n, uint x) internal pure returns (int) {
        uint result = f(uint256(n), x);
        return n > 0 ? int(result) : -int(result);
    }

    // Version 2
    function f(uint n, uint x) internal pure returns (uint) {
        return n + x;
    }
}

// ---- Expect: diagnostics ----
