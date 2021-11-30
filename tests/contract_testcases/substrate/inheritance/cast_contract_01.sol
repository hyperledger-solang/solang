
        interface IFoo {
            function bar(uint32) external pure returns (uint32);
        }

        contract foo  {
            function bar(IFoo x) public pure returns (uint32) {
                foo y = x;
            }
        }