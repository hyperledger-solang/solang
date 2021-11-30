
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int64 x) public override;
        }
        