contract example {
    modifier check_price(int64 price) {
        if (price >= 50) {
            _;
        }
    }

    function foo(int64 price) public check_price(price) {
        // ...
    }
}
