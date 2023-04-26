contract x {
            int override override y = 1;
        }
        
// ----
// error (38-46): duplicate 'override' attribute
// 	note (29-37): previous 'override' attribute
// error (38-46): only public variable can be declared 'override'
