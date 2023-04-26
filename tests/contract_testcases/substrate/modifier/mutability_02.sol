
        contract base {
            modifier foo() virtual {
                _;
            }
        }

        contract apex is base {
            function foo() public override {}
        }
// ----
// error (9-104): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract base'
// error (150-180): function 'foo' overrides modifier
// 	note (37-59): previous definition of 'foo'
