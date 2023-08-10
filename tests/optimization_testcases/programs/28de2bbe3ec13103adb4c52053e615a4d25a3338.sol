contract foo {
    function to_bytes(bytes4 b) public returns (bytes) {
        return b;
    }

    function to_bytes5(bytes b) public returns (bytes5) {
        return b;
    }
}
