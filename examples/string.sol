// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract String {
    string public value1;
    uint256 public value2;

    function setValue() public returns (string) {
        value1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        value2 = 4;

        string memory str = "asdsadsad";

        return str;
    }

    function getValue() public returns (string) {
        return value1;
    }
}
