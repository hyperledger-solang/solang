contract example {
    address owner;

    modifier only_owner() {
        require(msg.sender == owner);
        _;
        // insert post conditions here
    }

    function foo() public only_owner {
        // ...
    }
}
