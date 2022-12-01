
        interface I {
            function X(bytes) external;
        }

        function x() returns (bytes8) {
            return I.X.selector;
        }

        contract foo {}
        