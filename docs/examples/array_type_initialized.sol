contract primes {
    function primenumber(uint32 n) public pure returns (uint64) {
        uint64[10] primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];

        return primes[n];
    }
}
