
        contract c is b {
            constructor(int64 x) b(x+3) a(0) {}
        }

        abstract contract b is a(2) {
            constructor(int64 y) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }
