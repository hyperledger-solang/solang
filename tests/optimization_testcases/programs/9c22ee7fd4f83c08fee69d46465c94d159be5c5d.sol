
        @program_id("vS5Tf8mnHGbUCMLQWrnvsFvwHLfA5p3yQM3ozxPckn8")
        contract bar {
            @space(2 << 8 + 4)
            @seed("meh")
            @bump(33) // 33 = ascii !
            @payer(my_account)
            constructor() {}

            function hello() public returns (bool) {
                return true;
            }
        }
        