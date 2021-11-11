// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Dropnet {
    uint256 public value;

    function test() public {
        value = block.timestamp;
    }
}
