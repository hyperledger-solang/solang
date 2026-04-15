/// SPDX-License-Identifier: Apache-2.0

contract atomic_swap {
    function swap(
        address a,
        address b,
        address token_a,
        address token_b,
        uint64 amount_a,
        uint64 min_b_for_a,
        uint64 amount_b,
        uint64 min_a_for_b
    ) public {
        require(amount_b >= min_b_for_a, "not enough token B for token A");
        require(amount_a >= min_a_for_b, "not enough token A for token B");

        move_token(token_a, a, b, amount_a, min_a_for_b);
        move_token(token_b, b, a, amount_b, min_b_for_a);
    }

    function move_token(
        address token,
        address from,
        address to,
        uint64 max_spend_amount,
        uint64 transfer_amount
    ) internal {
        address contract_address = address(this);

        bytes payload = abi.encode("transfer", from, contract_address, max_spend_amount);
        (bool success, bytes returndata) = token.call(payload);

        payload = abi.encode("transfer", contract_address, to, transfer_amount);
        (success, returndata) = token.call(payload);

        payload = abi.encode(
            "transfer",
            contract_address,
            from,
            max_spend_amount - transfer_amount
        );
        (success, returndata) = token.call(payload);
    }
}
