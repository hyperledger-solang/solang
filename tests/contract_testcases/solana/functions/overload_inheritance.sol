
// All these cases should be allowed without errors
contract A {
    function foo(string d) pure public returns (uint32) {
        return d.length;
    }
}

contract B is A {
    function foo(bool c) pure public returns (uint32) {
        if (c)  {
            return 1;
        } else {
            return 2;
        }
    }
}

contract C is B {
    function foo(int d) pure public returns (uint32) {
        return uint32(d);
    }
}

contract D {
    function foo(bool f) pure public returns (uint32) {
        if (f) {
            return 3;
        } else {
            return 5;
        }
    } 
    function foo(int g) pure public returns (uint32) {
        return uint32(g);
    }
}

contract E is D {
    function foo(string h) pure public returns (uint32) {
        return h.length;
    }
}

contract F {
    function foo(string d) pure public returns (uint32) {
        return d.length;
    }
}

contract G {
    function foo(bool c) pure public returns (uint32) {
        if (c) {
            return 1;
        } else {
            return 2;
        }
    }
}

contract H is F, G {
    function foo(int d) pure public returns (uint32) {
        return uint32(d);
    }
}
// ---- Expect: diagnostics ----
