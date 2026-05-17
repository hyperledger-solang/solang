// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

contract Pause {
    bool private _isPaused;

    function paused() public view returns (bool) {
        return _isPaused;
    }

    function set(bool p) public {
        _isPaused = p;
    }
}