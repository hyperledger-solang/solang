
        contract c {
            /// @return a asda
            /// @return a barf
            function foo() public returns (int a, bool b) {}
        }
// ----
// error (70-77): duplicate tag '@return' for 'a'
