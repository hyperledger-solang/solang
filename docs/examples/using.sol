function mask(uint v, uint bits) returns (uint) {
    return v & ((1 << bits) - 1);
}

function odd(uint v) returns (bool) {
    return (v & 1) != 0;
}

contract c {
    using {mask, odd} for *;

    uint v;

    function set_v(uint n) public {
        v = n.mask(16);
    }
}
