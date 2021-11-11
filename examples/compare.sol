// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Compare {
    function compare(address t1, address t2) public view returns (bool) {
        return t1 < t2;
    }

    function check(address t) public view returns (uint160) {
        return uint160(t);
    }
}
