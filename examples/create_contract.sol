// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract B {
    uint256 public value;

    function getValue() public view returns (uint256) {
        return value;
    }

    function assignValue() public {
        value = 5;
    }
}

contract A {
    B public b;

    function init() public {
        b = new B();
    }

    function getValue() public view returns (uint256) {
        return b.getValue();
    }

    function assignValue() public {
        b.assignValue();
    }
}
