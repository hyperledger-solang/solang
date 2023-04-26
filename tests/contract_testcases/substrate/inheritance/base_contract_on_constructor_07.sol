
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
// ----
// error (9-84): missing arguments to base contract 'a' constructor
// error (67-71): duplicate base contract 'b'
// 	note (60-66): previous base contract 'b'
