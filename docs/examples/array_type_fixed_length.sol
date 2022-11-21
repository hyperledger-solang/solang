contract foo {
    /// In a vote with 11 voters, do the ayes have it?
    function f(bool[11] votes) public pure returns (bool) {
        uint32 i;
        uint32 ayes = 0;

        for (i = 0; i < votes.length; i++) {
            if (votes[i]) {
                ayes += 1;
            }
        }

        // votes.length is odd; integer truncation means that 11 / 2 = 5
        return ayes > votes.length / 2;
    }
}
