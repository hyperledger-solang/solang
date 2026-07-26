// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/mint-lock

contract mint_lock {
    mapping(address => uint64) limit;
    mapping(address => uint64) minted;

    function set_limit(address minter, uint64 max_amount) public {
        limit[minter] = max_amount;
    }

    function mint(address token, address minter, address to, uint64 amount) public {
        require(amount > 0, "amount must be positive");
        require(minted[minter] + amount <= limit[minter], "over mint limit");

        minted[minter] = minted[minter] + amount;

        bytes payload = abi.encode("mint", to, amount);
        (bool ok, bytes returndata) = token.call(payload);
    }
}
