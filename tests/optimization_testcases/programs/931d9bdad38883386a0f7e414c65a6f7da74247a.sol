contract Testing {
    function switch_default(uint a) public pure returns (uint b) {
        b = 4;
        assembly {
            switch a
            case 1 {
                b := 5
            }
            case 2 {
                b := 6
            }
            default {
                b := 7
            }
        }

        if (b == 7) {
            b += 2;
        }
    }

    function switch_no_default(uint a) public pure returns (uint b) {
        b = 4;
        assembly {
            switch a
            case 1 {
                b := 5
            }
            case 2 {
                b := 6
            }
        }

        if (b == 5) {
            b -= 2;
        }
    }

    function switch_no_case(uint a) public pure returns (uint b) {
        b = 7;
        assembly {
            switch a
            default {
                b := 5
            }
        }

        if (b == 5) {
            b -= 1;
        }
    }
}
