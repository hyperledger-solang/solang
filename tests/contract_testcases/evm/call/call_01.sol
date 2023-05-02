
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.staticcall{value: 2}("");
            }
        }

// ---- Expect: diagnostics ----
// error: 4:45-71: 'staticcall' cannot have value specified
