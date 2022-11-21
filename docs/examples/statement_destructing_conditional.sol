contract Foo {
    function test(bool cond) public {
        (int32 a, int32 b, int32 c) = cond ? (1, 2, 3) : (4, 5, 6);
    }
}
