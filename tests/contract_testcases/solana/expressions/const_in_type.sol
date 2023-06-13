
        contract x {
            uint public constant Y = 24;

            constructor(bytes32[Y] memory foo) {}
        }
// ---- Expect: diagnostics ----
// warning: 5:43-46: function parameter 'foo' is unused
