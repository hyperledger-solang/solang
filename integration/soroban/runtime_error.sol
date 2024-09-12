contract Error {
    uint64  count = 1;

    /// @notice Calling this function twice will cause an overflow
    function decrement() public returns (uint64){
        count -= 1;
        return count;
    }
}