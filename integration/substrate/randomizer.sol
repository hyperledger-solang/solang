contract randomizer {
    bytes32 public value;

    function get_random(bytes subject) public returns (bytes32) {
        bytes32 r1 = random(subject);
        // if we call random again in the same transaction, we should get the same result
        bytes32 r2 = random(subject);

        assert(r1 == r2);
        value = r1;
        return r1;
    }
}