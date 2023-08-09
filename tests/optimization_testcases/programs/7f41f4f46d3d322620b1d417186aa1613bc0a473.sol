pragma solidity 0;

contract foo {
    function test() public {
        bytes bar = new bytes(1);

        bar[0] = 128;

        assert(bar.length == 1);
        assert(128 == bar.pop());
        assert(bar.length == 0);
    }
}
