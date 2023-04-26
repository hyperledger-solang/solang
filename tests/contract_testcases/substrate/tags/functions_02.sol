
        contract c {
            /// @param f
            /**
             @param f asda
             */
            function foo(int f) public {}
        }
// ----
// error (77-83): duplicate tag '@param' for 'f'
