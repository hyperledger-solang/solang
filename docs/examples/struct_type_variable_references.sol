contract foo {
    struct bar {
        bytes32 f1;
        bytes32 f2;
        bytes32 f3;
        bytes32 f4;
    }

    function f(bar b) public {
        b.f4 = "foobar";
    }

    function example() public {
        bar bar1;

        // bar1 is passed by reference; just its pointer is passed
        f(bar1);

        assert(bar1.f4 == "foobar");
    }
}
