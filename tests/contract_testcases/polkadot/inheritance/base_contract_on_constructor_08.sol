
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

// ---- Expect: diagnostics ----
// error: 3:41-45: duplicate argument for base contract 'a'
// 	note 6:32-36: previous argument for base contract 'a'
// error: 6:32-36: duplicate argument for base contract 'a'
// 	note 6:32-36: previous argument for base contract 'a'
// warning: 7:31-32: function parameter 'y' is unused
