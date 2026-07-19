// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/custom_types
pragma solidity ^0.8.20;

contract CustomTypes {
    struct State {
        uint32 count;
        uint32 last_incr;
    }
    State state;

    function increment(uint32 incr) public returns (uint32) {
        state.count += incr;
        state.last_incr = incr;
        return state.count;
    }

    function get_state() public view returns (State memory) {
        return state;
    }
}
