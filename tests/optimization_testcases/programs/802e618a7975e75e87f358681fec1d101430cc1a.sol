pragma solidity 0;

contract foo {
    enum enum1 {
        val1,
        val2,
        val3
    }

    function test() public {
        enum1[] bar = new enum1[](1);

        bar[0] = enum1.val2;

        assert(bar.length == 1);
        assert(enum1.val2 == bar.pop());
        assert(bar.length == 0);
    }
}
