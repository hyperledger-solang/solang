
        interface I {
            function X(bytes) external;
        }

        function x() returns (bytes4) {
            return I.X.selector;
        }

        contract foo {}
        