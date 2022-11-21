contract example {
    int16 stored;

    function func(int256 x) public {
        if (x < type(int16).min || x > type(int16).max) {
            revert("value will not fit");
        }

        stored = int16(x);
    }
}
