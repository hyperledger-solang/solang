contract destructure {
    function func() internal returns (bool, int32, string) {
        return (true, 5, "abcd");
    }

    function test() public {
        string s;
        (bool b, , s) = func();
    }
}
