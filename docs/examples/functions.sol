// get_initial_bound is called from the constructor
function get_initial_bound() returns (uint256 value) {
    value = 102;
}

contract foo {
    uint256 bound = get_initial_bound();

    /** set bound for get with bound */
    function set_bound(uint256 _bound) public {
        bound = _bound;
    }

    // Clamp a value within a bound.
    // The bound can be set with set_bound().
    function get_with_bound(uint256 value) public view returns (uint256) {
        if (value < bound) {
            return value;
        } else {
            return bound;
        }
    }
}
