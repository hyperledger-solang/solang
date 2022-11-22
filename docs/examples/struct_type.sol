contract deck {
    enum suit {
        club,
        diamonds,
        hearts,
        spades
    }
    enum value {
        two,
        three,
        four,
        five,
        six,
        seven,
        eight,
        nine,
        ten,
        jack,
        queen,
        king,
        ace
    }
    struct card {
        value v;
        suit s;
    }

    function score(card c) public returns (uint32 score) {
        if (c.s == suit.hearts) {
            if (c.v == value.ace) {
                score = 14;
            }
            if (c.v == value.king) {
                score = 13;
            }
            if (c.v == value.queen) {
                score = 12;
            }
            if (c.v == value.jack) {
                score = 11;
            }
        }
        // all others score 0
    }
}
