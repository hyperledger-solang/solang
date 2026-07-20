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
        (bool ok, bytes memory ret) = pause_contract.call(abi.encode("paused"));
        require(ok, "pause check failed");
        bool is_paused = abi.decode(ret, (bool));
        // NOTE: if (is_paused) { revert ...; } crashes the compiler — bool from
        // abi.decode is not narrowed to i1 before the branch. Use require(!b) instead.
        require(!is_paused, "Paused");
        count += 1;
        return count;
    }
}
