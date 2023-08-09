pragma solidity 0;

contract foo {
    function test() public {
        int[] bar = (new int[])(1);

        bar[0] = 128;
        bar.push(64);

        assert(bar.length == 2);
        assert(bar[1] == 64);
    }
}
