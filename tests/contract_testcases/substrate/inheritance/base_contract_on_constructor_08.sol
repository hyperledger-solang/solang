
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

// ----
// error (67-71): duplicate argument for base contract 'a'
// 	note (117-121): previous argument for base contract 'a'
// error (117-121): duplicate argument for base contract 'a'
// 	note (117-121): previous argument for base contract 'a'
// warning (154-155): function parameter 'y' has never been read
