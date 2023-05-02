contract Base {
    uint256 public constant FOO = 1;
    uint256 public something = 0;

    function nop() public pure {}

    function set(uint256 val) public virtual {
        something = val;
    }
}

contract A {
    function a() public {
        if (Base.FOO == 1) {
            print("hi");
        }
    }
}

contract B {
    function b() public {
        Base.set(1);
    }
}

contract C is Base {
    function c() public {
        if (Base.FOO == 1) {
            Base.set(1);
        }
    }

    function set(uint256 val) public virtual override {
        something = val + 1024;
    }
}

// ---- Expect: diagnostics ----
// error: 22:9-20: function calls via contract name are only valid for base contracts
