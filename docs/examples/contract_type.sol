contract child {
    function announce() public {
        print("Greetings from child contract");
    }
}

contract creator {
    function test() public {
        child c = new child();

        c.announce();
    }
}
