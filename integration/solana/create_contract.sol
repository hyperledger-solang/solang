
contract creator {
    child public c;

    function create_child() public {
        print("Going to create child");
        c = new child();

        c.say_hello();
    }
}

contract child {
    constructor() {
        print("In child constructor");
    }

    function say_hello() pure public {
        print("Hello there");
    }
}
