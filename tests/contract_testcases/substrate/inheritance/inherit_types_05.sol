
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
        
// ----
// error (9-124): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract b'
// error (41-44): already defined 'foo'
// 	note (164-167): previous definition of 'foo'
