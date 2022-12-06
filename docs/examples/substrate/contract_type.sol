contract child {
    function announce() public {
        print("Greetings from child contract");
    }
}

contract creator {
    function test() public {
        // Note: on Solana, new Contract() requires an address
        child c = new child();

        c.announce();
    }
}
