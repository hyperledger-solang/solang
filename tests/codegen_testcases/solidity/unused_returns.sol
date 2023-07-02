// RUN: --target polkadot --emit cfg
contract C {
        function foo() public returns (int, int) { return (1, 2); }

        // BEGIN-CHECK: C::function::bar
        function bar() public { 
            // NOT-CHECK: abidecode
            this.foo(); 

            function () external returns (int, int) fPtr = this.foo;

            // NOT-CHECK: abidecode
            fPtr();
        }
}
