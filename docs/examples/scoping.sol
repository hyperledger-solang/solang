contract Foo {
    function foo() public {
        // new block is introduced with { and ends with }
        {
            uint256 a;

            a = 102;
        }

        // ERROR: a is out of scope
        // uint256 b = a + 5;
    }
}
