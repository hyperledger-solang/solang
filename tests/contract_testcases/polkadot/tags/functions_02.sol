
        contract c {
            /// @param f
            /**
             @param f asda
             */
            function foo(int f) public {}
        }
// ---- Expect: diagnostics ----
// warning: 5:14-27: duplicate tag '@param' for 'f'
// 	note 3:17-25: previous tag '@param' for 'f'
// warning: 7:13-39: function can be declared 'pure'
// warning: 7:30-31: function parameter 'f' is unused
