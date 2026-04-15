contract liquidity_pool {
    uint64 public total_shares;
    uint64 public reserve_a;
    uint64 public reserve_b;
    mapping(address => uint64) public shares;

    function balance_shares(address user) public view returns (uint64) {
        return shares[user];
    }

    function deposit(
        address to,
        address token_a,
        address token_b,
        uint64 desired_a,
        uint64 min_a,
        uint64 desired_b,
        uint64 min_b
    ) public {
        to.requireAuth();

        uint64 amount_a = desired_a;
        uint64 amount_b = desired_b;

        if (!(reserve_a == 0 && reserve_b == 0)) {
            uint64 optimal_b = (desired_a * reserve_b) / reserve_a;
            if (optimal_b <= desired_b) {
                require(optimal_b >= min_b, "amount_b less than min");
                amount_b = optimal_b;
            } else {
                uint64 optimal_a = (desired_b * reserve_a) / reserve_b;
                require(optimal_a <= desired_a && optimal_a >= min_a, "amount_a invalid");
                amount_a = optimal_a;
            }
        }

        require(amount_a > 0 && amount_b > 0, "both amounts must be > 0");

        token_transfer(token_a, to, address(this), amount_a);
        token_transfer(token_b, to, address(this), amount_b);

        uint64 balance_a = token_balance(token_a, address(this));
        uint64 balance_b = token_balance(token_b, address(this));

        uint64 new_total_shares = total_shares;
        if (reserve_a > 0 && reserve_b > 0) {
            uint64 shares_a = (balance_a * total_shares) / reserve_a;
            uint64 shares_b = (balance_b * total_shares) / reserve_b;
            new_total_shares = shares_a < shares_b ? shares_a : shares_b;
        } else {
            new_total_shares = isqrt(balance_a * balance_b);
        }

        require(new_total_shares >= total_shares, "invalid share growth");

        uint64 minted = new_total_shares - total_shares;
        shares[to] = shares[to] + minted;
        total_shares = total_shares + minted;
        reserve_a = balance_a;
        reserve_b = balance_b;
    }

    function swap_buy_a(
        address to,
        address token_a,
        address token_b,
        uint64 out,
        uint64 in_max
    ) public {
        swap_internal(to, token_a, token_b, true, out, in_max);
    }

    function swap_buy_b(
        address to,
        address token_a,
        address token_b,
        uint64 out,
        uint64 in_max
    ) public {
        swap_internal(to, token_a, token_b, false, out, in_max);
    }

    function swap_internal(
        address to,
        address token_a,
        address token_b,
        bool buy_a,
        uint64 out,
        uint64 in_max
    ) internal {
        to.requireAuth();

        uint64 sell_reserve = buy_a ? reserve_b : reserve_a;
        uint64 buy_reserve = buy_a ? reserve_a : reserve_b;

        require(buy_reserve > out, "not enough token to buy");

        uint64 n = sell_reserve * out * 1000;
        uint64 d = (buy_reserve - out) * 997;
        uint64 sell_amount = (n / d) + 1;
        require(sell_amount <= in_max, "in amount is over max");

        address sell_token = buy_a ? token_b : token_a;
        address buy_token = buy_a ? token_a : token_b;

        token_transfer(sell_token, to, address(this), sell_amount);
        token_transfer(buy_token, address(this), to, out);

        reserve_a = token_balance(token_a, address(this));
        reserve_b = token_balance(token_b, address(this));
        require(reserve_a > 0 && reserve_b > 0, "new reserves must be > 0");
    }

    function withdraw(
        address to,
        address token_a,
        address token_b,
        uint64 share_amount,
        uint64 min_a,
        uint64 min_b
    ) public {
        to.requireAuth();
        require(shares[to] >= share_amount, "insufficient shares");
        require(total_shares > 0, "no total shares");

        uint64 balance_a = token_balance(token_a, address(this));
        uint64 balance_b = token_balance(token_b, address(this));

        uint64 out_a = (balance_a * share_amount) / total_shares;
        uint64 out_b = (balance_b * share_amount) / total_shares;

        require(out_a >= min_a && out_b >= min_b, "min not satisfied");

        shares[to] = shares[to] - share_amount;
        total_shares = total_shares - share_amount;

        token_transfer(token_a, address(this), to, out_a);
        token_transfer(token_b, address(this), to, out_b);

        reserve_a = token_balance(token_a, address(this));
        reserve_b = token_balance(token_b, address(this));
    }

    function token_transfer(address token, address from, address to, uint64 amount) internal {
        bytes payload = abi.encode("transfer", from, to, amount);
        (bool success, bytes memory returndata) = token.call(payload);
        success;
        returndata;
    }

    function token_balance(address token, address owner) internal returns (uint64) {
        bytes payload = abi.encode("balance", owner);
        (bool success, bytes memory returndata) = token.call(payload);
        success;
        return abi.decode(returndata, (uint64));
    }

    function isqrt(uint64 x) internal pure returns (uint64) {
        if (x == 0) {
            return 0;
        }

        uint64 y = x;
        uint64 z = (x + 1) / 2;
        while (z < y) {
            y = z;
            z = (x / z + z) / 2;
        }

        return y;
    }
}
