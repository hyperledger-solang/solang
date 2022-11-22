contract Foo {
    function test() public {
        (int32 a, int32 b, int32 c) = (1, 2, 3);

        (b, , a) = (a, 5, b);
    }
}
