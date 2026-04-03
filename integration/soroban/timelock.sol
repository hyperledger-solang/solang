contract timelock {
    enum TimeBoundKind {
        Before,
        After
    }

    enum BalanceState {
        Uninitialized,
        Funded,
        Claimed
    }

    BalanceState public state;
    TimeBoundKind mode;
    uint64 public amount;
    uint64 public bound_timestamp;

    function deposit(
        address from,
        address token_,
        uint64 amount_,
        TimeBoundKind mode_,
        uint64 bound_timestamp_
    ) public {
        require(
            state == BalanceState.Uninitialized,
            "contract has been already initialized"
        );

        from.requireAuth();

        amount = amount_;
        mode = mode_;
        bound_timestamp = bound_timestamp_;

        bytes payload = abi.encode("transfer", from, address(this), amount_);
        token_.call(payload);

        state = BalanceState.Funded;
    }

    function claim(address token_, address claimant) public {
        claimant.requireAuth();

        require(state == BalanceState.Funded, "balance is not claimable");
        require(check_time_bound(), "time predicate is not fulfilled");

        state = BalanceState.Claimed;

        bytes memory payload = abi.encode("transfer", address(this), claimant, amount);
        token_.call(payload);
    }

    function now_ts() public view returns (uint64) {
        return block.timestamp;
    }

    function check_time_bound() internal view returns (bool) {
        if (mode == TimeBoundKind.After) {
            return block.timestamp >= bound_timestamp;
        }

        return block.timestamp <= bound_timestamp;
    }
}
