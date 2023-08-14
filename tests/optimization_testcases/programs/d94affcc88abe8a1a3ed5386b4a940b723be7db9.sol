pragma solidity 0;

contract foo {
    function test() public {
        int[] bar = new int[](1);

        bar[0] = 128;

        assert(bar.length == 1);
        assert(128 == bar.pop());
        assert(bar.length == 0);
    }
}
