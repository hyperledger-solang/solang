// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/increment_with_pause
pragma solidity ^0.8.20;

contract IncrementContract {
    address public instance pause_contract;
    uint32 public instance count = 0;

    constructor(address _pause) {
        pause_contract = _pause;
    }

    function increment() public returns (uint32) {
        // Cross-contract call into the pause contract's `paused()` function.
        bytes payload = abi.encode("paused");
        (, bytes memory ret) = pause_contract.call(payload);
        bool is_paused = abi.decode(ret, (bool));
        require(!is_paused, "Paused");

        count += 1;
        // Extend the instance storage TTL: bump to 100 ledgers if it drops below 50.
        extendInstanceTtl(50, 100);
        return count;
    }
}
