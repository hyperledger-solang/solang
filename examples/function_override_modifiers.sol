abstract contract base {
    address owner;

    modifier only_owner() {
        require(msg.sender == owner);
        _;
    }

    modifier check_price(int64 price) virtual {
        if (price >= 10) {
            _;
        }
    }
}

contract example is base {
    modifier check_price(int64 price) override {
        if (price >= 50) {
            _;
        }
    }

    function foo(int64 price) public only_owner check_price(price) {
        // ...
    }
}
