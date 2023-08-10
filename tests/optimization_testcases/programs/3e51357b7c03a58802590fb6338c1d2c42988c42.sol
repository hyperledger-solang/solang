contract test {
    function testing(bytes memory data) public pure {
        string[4] arr = abi.decode(data, (string[4]));
        assert(arr[0] == "a");
        assert(arr[1] == "b");
        assert(arr[2] == "c");
        assert(arr[3] == "d");
    }
}
