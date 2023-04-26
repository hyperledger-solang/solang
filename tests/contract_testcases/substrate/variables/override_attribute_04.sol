contract x is y {
            int public foo;
        }

        contract y {
            function foo() public virtual returns (int) {
                return 102;
            }
        }
        
// ----
// error (90-133): function 'foo' with this signature already defined
// 	note (41-44): previous definition of function 'foo'
