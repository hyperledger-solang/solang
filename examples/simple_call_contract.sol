// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract B {
    uint256 public a;

    function setA(uint256 _a) public {
        a = _a;
    }

    function getA() public view returns (uint256) {
        return a;
    }
}

contract A {
    B b;

    uint256 public value;

    function init(address _b) public {
        b = B(_b);
    }

    function getA() public view returns (uint256) {
        return b.getA();
    }
}
