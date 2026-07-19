// SPDX-License-Identifier: MIT
// Mapping of https://github.com/stellar/soroban-examples/tree/main/events
pragma solidity ^0.8.20;

contract IncrementContract {
    uint32 public instance count = 0;
    event IncrementEvent(string indexed action, string indexed method, uint32 count);

    function increment() public returns (uint32) {
        count += 1;
        emit IncrementEvent("COUNTER", "increment", count);
        return count;
    }
}
