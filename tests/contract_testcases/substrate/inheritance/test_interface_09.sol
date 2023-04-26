
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
        
// ----
// warning (202-231): function can be declared 'pure'
// warning (222-223): function parameter 'a' has never been read
// warning (290-319): function can be declared 'pure'
// warning (310-311): function parameter 'a' has never been read
