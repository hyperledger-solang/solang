
        abstract contract base {
            constructor(bool x) {}
        }

        contract apex is base {
                function foo() pure public {}
        }
// ----
// warning (63-64): function parameter 'x' has never been read
// error (88-167): missing arguments to base contract 'base' constructor
