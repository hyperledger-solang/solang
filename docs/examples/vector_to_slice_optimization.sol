contract test {
    function can_be_slice() public {
        // v can just be a pointer to constant memory and an a length indicator
        string v = "Hello, World!";

        print(v);
    }

    function must_be_vector() public {
        // if v is a vector, then it needs to allocated and default value copied.
        string v = "Hello, World!";

        // bs is copied by reference is now modifyable
        bytes bs = bytes(v);

        bs[1] = 97;

        print(v);
    }
}
