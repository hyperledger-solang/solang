
        interface IFoo {
            function bar(uint32) external pure returns (uint32);
        }

        contract foo  {
            function bar(IFoo x) public pure returns (uint32) {
                foo y = x;
            }
        }
// ---- Expect: diagnostics ----
// error: 8:25-26: implicit conversion not allowed since contract foo is not a base contract of contract IFoo
