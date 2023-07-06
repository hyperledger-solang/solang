
        /// @param f asdad
        /// @param f bar
        event x (
            uint32 f
        );
// ---- Expect: diagnostics ----
// warning: 3:13-25: duplicate tag '@param' for 'f'
// 	note 2:13-27: previous tag '@param' for 'f'
// warning: 4:15-16: event 'x' has never been emitted
