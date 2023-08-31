// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.0;

import "A.sol";

contract Contract {
    function use_a(A a) public pure {
        a.add(1, 1);
    }
}
