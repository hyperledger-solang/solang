type Value is uint128;

function inc_and_wrap(int128 v) returns (Value) {
    return Value.wrap(uint128(v + 1));
}

function dec_and_unwrap(Value v) returns (uint128) {
    return Value.unwrap(v) - 1;
}
