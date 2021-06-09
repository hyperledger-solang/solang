
contract creator {
    child public c;

    function child_as_u256() public view returns (uint) {
        return uint(address(c));
    }

    function create_child() public {
        c = new child();
    }

    function call_child() public pure returns (string) {
        return c.say_my_name();
    }
}

contract child {
    function say_my_name() pure public returns (string) {
        return "child";
    }
}