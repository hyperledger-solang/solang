// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Value {
    uint256 public value;

    address public ad;

    function r() external payable {
        value = msg.value;
    }

    function getBalance(address a) public pure returns (uint256) {
        return a.balance;
    }

    function getSender() public {
        ad = msg.sender;
    }

    function getOrigin() public  {
        ad = tx.origin;
    }

    function getAddress() public returns (address) {
        return address(this);
    }

    function send(address payable a, uint256 v) public {
        (,) = address(a).call{value: v}("");
    }
}
