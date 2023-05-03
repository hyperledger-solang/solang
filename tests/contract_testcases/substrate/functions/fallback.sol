
        contract test {
            int64 result = 102;

            function get() public returns (int64) {
                return result;
            }

            function() external {
                result = 356;
            }
        }
// ---- Expect: diagnostics ----
// error: 9:13-32: function is missing a name. A function without a name is syntax for 'fallback() external' or 'receive() external' in older versions of the Solidity language, see https://solang.readthedocs.io/en/latest/language/functions.html#fallback-and-receive-function
