/// SPDX-License-Identifier: Apache-2.0

contract liquidity_pool_token_a {
    mapping(address => uint64) public balances;

    function mint(address to, uint64 amount) public {
        balances[to] = balances[to] + amount;
    }

    function transfer(address from, address to, uint64 amount) public {
        require(balances[from] >= amount, "Insufficient balance");
        balances[from] = balances[from] - amount;
        balances[to] = balances[to] + amount;
    }

    function balance(address owner) public view returns (uint64) {
        return balances[owner];
    }
}
