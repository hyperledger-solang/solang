
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int x) public { print ("foo"); }
        }
        