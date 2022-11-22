contract foo {
    function bar() public {
        other o = new other();

        o.feh{value: 102, gas: 5000}(102);
    }
}

contract other {
    function feh(uint32 x) public payable {
        // ...
    }
}
