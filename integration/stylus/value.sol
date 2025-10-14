// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

contract C {
    function test() public payable returns (uint256) {
        return this.other{value: msg.value}();
    }

    function other() public payable returns (uint256) {
        uint256 value = msg.value;
        print("value = {}".format(value));
        assert(value == 1000000000000000000);
        return value;
    }
}
