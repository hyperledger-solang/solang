type Value is uint128;

contract Foo {
    function inc_and_wrap(int128 v) public returns (Value) {
        return Value.wrap(uint128(v + 1));
    }

    function dec_and_unwrap(Value v) public returns (uint128) {
        return Value.unwrap(v) - 1;
    }
}
