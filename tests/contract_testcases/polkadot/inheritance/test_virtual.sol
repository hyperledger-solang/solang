
        contract c {
            function test() public;
        }
// ---- Expect: diagnostics ----
// error: 3:13-35: function with no body missing 'virtual'. This was permitted in older versions of the Solidity language, please update.
