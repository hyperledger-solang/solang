contract example {
    struct user {
        address who;
        uint32 hitcount;
    }
    user[] foo;

    function test() public {
        // foo.push() creates an empty entry and returns a reference to it
        user storage x = foo.push();

        x.who = address(1);
        x.hitcount = 1;
    }
}
