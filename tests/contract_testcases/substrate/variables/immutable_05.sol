contract x {
            int immutable public immutable y = 1;
        }
        
// ----
// error (46-55): duplicate 'immutable' attribute
// 	note (29-38): previous 'immutable' attribute
