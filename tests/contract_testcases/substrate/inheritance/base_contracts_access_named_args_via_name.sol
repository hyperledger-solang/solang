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

// ----
// error (178-196): function calls via contract name are only valid for base contracts
