enum planets {
    Mercury,
    Venus,
    Earth,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune
}

abstract contract timeofday {
    enum time {
        Night,
        Day,
        Dawn,
        Dusk
    }
}

contract stargazing {
    function look_for(timeofday.time when) public returns (planets[]) {
        if (when == timeofday.time.Dawn || when == timeofday.time.Dusk) {
            planets[] x = new planets[](2);
            x[0] = planets.Mercury;
            x[1] = planets.Venus;
            return x;
        } else if (when == timeofday.time.Night) {
            planets[] x = new planets[](5);
            x[0] = planets.Mars;
            x[1] = planets.Jupiter;
            x[2] = planets.Saturn;
            x[3] = planets.Uranus;
            x[4] = planets.Neptune;
            return x;
        } else {
            planets[] x = new planets[](1);
            x[0] = planets.Earth;
            return x;
        }
    }
}
