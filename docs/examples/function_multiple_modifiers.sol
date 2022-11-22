contract example {
    address owner;

    // a modifier with no arguments does not need "()" in its declaration
    modifier only_owner() {
        require(msg.sender == owner);
        _;
    }

    modifier check_price(int64 price) {
        if (price >= 50) {
            _;
        }
    }

    function foo(int64 price) public only_owner check_price(price) {
        // ...
    }
}
