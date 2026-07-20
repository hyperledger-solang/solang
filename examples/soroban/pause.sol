// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/pause
pragma solidity ^0.8.20;

contract Pause {
    bool instance paused_flag = false;

    function paused() public view returns (bool) {
        return paused_flag;
    }

    function set(bool paused) public {
        paused_flag = paused;
    }
}
