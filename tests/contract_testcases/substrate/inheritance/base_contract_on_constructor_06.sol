
        contract c is b {
            constructor(int64 x) b(x+3) {}
        }

        abstract contract b is a {
            constructor(int64 y) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }
// ---- Expect: diagnostics ----
// error: 2:9-4:10: missing arguments to base contract 'a' constructor
// warning: 7:31-32: function parameter 'y' is unused
