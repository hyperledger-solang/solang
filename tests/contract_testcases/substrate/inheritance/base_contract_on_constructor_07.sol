
        contract c is b {
            constructor(int64 x) b(x+3) b(0) {}
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
// error: 3:41-45: duplicate base contract 'b'
// 	note 3:34-40: previous base contract 'b'
