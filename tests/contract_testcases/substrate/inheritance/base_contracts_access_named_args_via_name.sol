contract Base {
    uint256 private something;

    function set(uint256 val) public pure {
        something = val;
    }
}

contract A {
    function a() public pure {
        Base.set({val: 1});
    }
}

contract B is Base {
    function b() public pure {
        Base.set({val: 1});
    }
}

// ---- Expect: diagnostics ----
// error: 11:9-27: function calls via contract name are only valid for base contracts
