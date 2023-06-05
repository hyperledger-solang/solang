contract test {
            function foobar(int32 foo, uint16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-73: function can be declared 'pure'
// warning: 2:35-38: function parameter 'foo' is unused
