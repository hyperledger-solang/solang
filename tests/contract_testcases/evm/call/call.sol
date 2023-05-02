
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.delegatecall{value: 2}("");
            }
        }

// ---- Expect: diagnostics ----
// error: 4:45-73: 'delegatecall' cannot have value specified
