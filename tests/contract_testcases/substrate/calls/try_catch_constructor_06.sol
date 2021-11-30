
        contract c {
            function test(other o) public {
                try o.test() {
                    x = 1;
                } {
                    x= 5;
                }
                catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public  {
            }
        }
        