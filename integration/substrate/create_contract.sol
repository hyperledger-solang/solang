contract creator {
    child_create_contract public c;

    function create_child() public {
        c = new child_create_contract{value: 1e15}();
    }

    function call_child() public view returns (string memory) {
        return c.say_my_name();
    }
}

contract child_create_contract {
    constructor() payable {}

    function say_my_name() public pure returns (string memory) {
        return "child";
    }
}
