contract enum_example {
    enum Weekday {
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday,
        Sunday
    }

    function is_weekend(Weekday day) public pure returns (bool) {
        return (day == Weekday.Saturday || day == Weekday.Sunday);
    }
}
