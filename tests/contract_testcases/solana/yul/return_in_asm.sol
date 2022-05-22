contract Contract {
    constructor() {
        assembly ("memory-safe") {
            return(0, 0)
        }
    }
}
