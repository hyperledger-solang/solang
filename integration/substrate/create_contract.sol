
contract creator {
    child public c;

    function create_child() public {
        c = new child{value: 1e15}();
    }

    function call_child() public view returns (string memory) {
        return c.say_my_name();
    }
}

contract child {
    constructor() payable {}

    function say_my_name() pure public returns (string memory) {
        return "child";
    }
}