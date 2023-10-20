library InstanceLibrary {
    function getMax(uint64 a, uint64 b) external pure returns (uint64) {
        return a > b ? a : b; // If 'a' is greater than 'b', it returns 'a'; otherwise, it returns 'b'.
    }
}

contract TestContract {
    using InstanceLibrary for uint64;

    // Calculate and return the maximum value between x and 65536 using the InstanceLibrary.
    function calculateMax(uint64 x) public pure returns (uint64) {
        return x.getMax(65536);
    }
}
