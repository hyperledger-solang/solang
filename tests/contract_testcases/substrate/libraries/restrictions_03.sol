
        library c {
            function f() public payable {}
            modifier m() override {}
            modifier m2();
        }
// ----
// error (33-60): function in a library cannot be payable
// error (89-97): modifier in a library cannot override
// error (113-126): modifier in a library must have a body
