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

    card card1 = card(value.two, suit.club);
    card card2 = card({s: suit.club, v: value.two});

    // This function does a lot of copying
    function set_card1(card c) public returns (card previous) {
        previous = card1;
        card1 = c;
    }
}
