// SPDX-License-Identifier: Apache-2.0
// Minimal token used by the mint_lock example
// (https://github.com/stellar/soroban-examples/tree/main/mint-lock).

contract ml_token {
    mapping(address => uint64) public balances;

    function mint(address to, uint64 amount) public {
        balances[to] = balances[to] + amount;
    }

    function balance(address addr) public view returns (uint64) {
        return balances[addr];
    }
}
