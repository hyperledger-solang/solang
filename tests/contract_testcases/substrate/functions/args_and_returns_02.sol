
    contract args {
        function foo(bool arg1, uint arg2) public returns (address, uint) {
        }
    }
// ----
// warning (29-94): function can be declared 'pure'
// warning (47-51): function parameter 'arg1' has never been read
// warning (58-62): function parameter 'arg2' has never been read
