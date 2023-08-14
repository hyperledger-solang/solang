pragma solidity 0;

contract foo {
    enum enum1 {
        val1,
        val2,
        val3
    }

    function test() public {
        enum1[] bar = new enum1[](1);

        bar[0] = enum1.val1;
        bar.push(enum1.val2);

        assert(bar.length == 2);
        assert(bar[1] == enum1.val2);
    }
}
