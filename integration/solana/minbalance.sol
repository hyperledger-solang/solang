import '../../solana-library/minimum_balance.sol';

contract minbalance {
    /// Anchor requires view functions to have a return value
    function test1() pure public returns (bool) {
        require(DEFAULT_LAMPORTS_PER_BYTE_YEAR == 3480, "lamports per byte year");
        require(DEFAULT_EXEMPTION_THRESHOLD == 2, "lamports per byte year");
        require(ACCOUNT_STORAGE_OVERHEAD == 128, "storage overhead");

        return true;
    }

    function test2(uint64 space) public pure returns (uint64) {
        return minimum_balance(space);
    }
}