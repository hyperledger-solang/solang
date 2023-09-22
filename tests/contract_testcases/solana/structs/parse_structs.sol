contract hatchling {
    struct A {
        A [][2][1] b;
    }
    struct B {
        B [2][][1] b;
    }
    struct C {
        C [2][1][] b;
    }

    A private n1;
    B private n2;
    C private n3;

    constructor() {}

    function foo(uint a, uint b) public {

    }
}

// ---- Expect: diagnostics ----
// warning: 12:5-17: storage variable 'n1' has never been used
// warning: 13:5-17: storage variable 'n2' has never been used
// warning: 14:5-17: storage variable 'n3' has never been used
// warning: 18:5-40: function can be declared 'pure'
// warning: 18:23-24: function parameter 'a' is unused
// warning: 18:31-32: function parameter 'b' is unused