
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.call{value: 2}("");
            }
        }

// ---- Expect: diagnostics ----
// warning: 4:23-24: destructure variable 's' has never been used
// warning: 4:39-41: destructure variable 'bs' has never been used
