
        contract base {
            modifier foo() virtual {
                _;
            }
        }

        contract apex is base {
            function foo() public override {}
        }