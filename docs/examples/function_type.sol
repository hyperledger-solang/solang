contract ft {
    function test() public {
        // reference to an internal function with two argments, returning bool
        // with the default mutability (i.e. cannot be payable)
        function(int32, bool) internal returns (bool) x;

        // the local function func1 can be assigned to this type; mutability
        // can be more restrictive than the type.
        x = func1;

        // now you can call func1 via the x
        bool res = x(102, false);

        // reference to an internal function with no return values, must be pure
        function(int32, bool) internal pure y;

        // Does not compile:
        // Wrong number of return types and mutability is not compatible
        // y = func1;
    }

    function func1(int32 arg, bool arg2) internal view returns (bool) {
        return false;
    }
}
