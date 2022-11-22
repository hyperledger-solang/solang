contract test {
    function f() public {
        for (uint256 i = 0; i < 10; i++) {
            // this multiply can be done with a 64 bit instruction
            g(i * 100);
        }
    }

    function g(uint256 v) internal {
        // ...
    }
}
