abstract contract test {
            address foo = address(0xA368dF6DFCD5Ba7b0BC108AF09e98E4655e35A2c3B2e2D5E3Eae6c6f7CD8D2D4);

            function bar() private returns (bool) {
                return foo > address(0);
            }
        }
// ---- Expect: diagnostics ----
// warning: 4:13-50: function can be declared 'view'
