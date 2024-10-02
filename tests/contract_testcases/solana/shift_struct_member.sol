// Test case for https://github.com/hyperledger-solang/solang/issues/732
library FixedPoint {

    struct uq144x112 {
        uint _x;
    }

    struct uq112x112 {
        uint224 _x;
    }

    // this errors with `type not allowed Ref(Uint(224))`
    function decode(uq112x112 memory self) internal pure returns (uint112) {
        return uint112(self._x >> 112);
    }

    // this error with `type not allowed Ref(Uint(256))`
    function decode144(uq144x112 memory self) internal pure returns (uint144) {
        return uint144(self._x >> 112);
    }
}

// ---- Expect: diagnostics ----
