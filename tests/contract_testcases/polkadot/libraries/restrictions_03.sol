
        library c {
            function f() public payable {}
            modifier m() override {}
            modifier m2();
        }
// ---- Expect: diagnostics ----
// error: 3:13-40: function in a library cannot be payable
// error: 4:26-34: modifier in a library cannot override
// error: 5:13-26: modifier in a library must have a body
