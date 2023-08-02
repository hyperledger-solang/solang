pragma solidity 0;

contract foo {
    function test() public returns (int) {
        int[] bar = new int[](0);
        return bar.pop();
    }
}
