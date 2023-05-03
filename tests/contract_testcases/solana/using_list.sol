function id(uint x) pure returns (uint) {
    return x;
}

function zero(uint) pure returns (uint) {
    return 0;
}

contract C {
    function f(uint z) pure external returns(uint) {
        return z.id();
    }

    function g(uint z) pure external returns (uint) {
        return z.zero();
    }

    using {id, zero} for uint;
}


// ---- Expect: diagnostics ----
