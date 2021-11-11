// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract C {
    uint256 public value;

    function getValue() public view returns (uint256) {
        return value;
    }

    function assignValue() public {
        //value = 5;
    }
}

contract B {
    uint256 public value;

    address public c;

    function init(address _c) public {
        c = _c;
    }

    function getValue() public view returns (uint256) {
        return value;
    }

    function assignValue() public {
        (bool success, bytes memory result) = address(c).delegatecall(abi.encodeWithSignature("assignValue()"));
    }
}

contract A {
    uint256 public value;

    address public b;

    function init(address _b) public {
        b = _b;
    }

    function getValue() public view returns (uint256) {
        return value;
    }

    function assignValue() public {
        (bool success, bytes memory result) = address(b).staticcall(abi.encodeWithSignature("assignValue()"));
    }
}
