contract foo {
    function assert_fails() public {
        require(true, "humpty-dumpty");
    }
}
