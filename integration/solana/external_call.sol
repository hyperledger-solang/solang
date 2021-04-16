
contract caller {
    function do_call(callee e, int64 v) public {
        e.set_x(v);
    }
}

contract callee {
    int64 x;

    function set_x(int64 v) public {
        x = v;
    }

    function get_x() public view returns (int64) {
        return x;
    }
}