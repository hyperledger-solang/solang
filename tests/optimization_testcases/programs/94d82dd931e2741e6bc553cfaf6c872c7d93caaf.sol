pragma solidity 0;

contract foo {
    function test() public {
        int[] bar = new int[](1);
        bar[0] = 12;
        bar.pop();

        assert(bar[0] == 12);
    }
}
