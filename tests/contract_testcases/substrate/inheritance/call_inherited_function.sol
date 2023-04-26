
        abstract contract base {
            function foo() private returns (uint64) {
                return 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo() + 3;
            }
        }
        
// ----
// error (249-254): cannot call private function
// 	note (46-85): declaration of function 'foo'
