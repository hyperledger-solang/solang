// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

interface IPause {
    function paused() external view returns (bool);
}

contract IncrementContract {
    IPause private instance pauseContract;
    uint32 private instance count;

    error PausedError();

    constructor(IPause _pause) {
        pauseContract = _pause;
    }

    function increment() public returns (uint32) {
        // Cross-contract call to check the paused state
        if (pauseContract.paused()) {
            revert PausedError();
        }

        count += 1;
        
        return count;
    }
}