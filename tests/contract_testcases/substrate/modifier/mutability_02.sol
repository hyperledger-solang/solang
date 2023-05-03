
        contract base {
            modifier foo() virtual {
                _;
            }
        }

        contract apex is base {
            function foo() public override {}
        }
// ---- Expect: diagnostics ----
// error: 2:9-6:10: contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract base'
// error: 9:13-43: function 'foo' overrides modifier
// 	note 3:13-35: previous definition of 'foo'
