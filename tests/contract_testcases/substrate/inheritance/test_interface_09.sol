
        interface bar {
            function f1(address a) external;
        }

        interface bar2 {
            function f1(address a) external;
        }

        contract x is bar {
            function f1(address a) public {}
        }

        contract y is bar2, x {
            function f2(address a) public {}
        }
        