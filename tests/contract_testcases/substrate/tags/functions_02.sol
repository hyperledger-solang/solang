
        contract c {
            /// @param f
            /**
             @param f asda
             */
            function foo(int f) public {}
        }
// ---- Expect: diagnostics ----
// error: 5:15-21: duplicate tag '@param' for 'f'
