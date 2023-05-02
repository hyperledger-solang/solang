
        contract b {
            struct foo {
                uint32 f1;
                uint32 f2;
            }
        }

        contract c {
            enum foo { f1, f2 }
        }

        contract a is b, c {
            function test(foo x) public {
            }
        }
        
// ---- Expect: diagnostics ----
// error: 2:9-7:10: contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract b'
// error: 3:20-23: already defined 'foo'
// 	note 10:18-21: previous definition of 'foo'
