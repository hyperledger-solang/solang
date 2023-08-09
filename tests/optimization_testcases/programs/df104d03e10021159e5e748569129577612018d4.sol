contract foo {
    function return_true() public returns (bool) {
        return true;
    }

    function return_false() public returns (bool) {
        return false;
    }

    function true_arg(bool b) public {
        assert(b);
    }

    function false_arg(bool b) public {
        assert(!b);
    }
}
