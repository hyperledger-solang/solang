// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract B {
    uint256 public value;
}

contract A {
    function t1() public returns (address) {
        uint256 salt = uint256(keccak256("asdasd"));
        return address(new B{salt: salt}());
    }

    function t2() public returns (address) {
        return address(uint(keccak256(abi.encodePacked(
            hex'ff',
            address(this),
            keccak256("asdasd"),
            keccak256(type(B).creationCode)
        ))));
    }
}
