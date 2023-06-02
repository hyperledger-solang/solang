FROM paritytech/contracts-ci-linux:c531ca3a-20230602

# This version of the contracts node contains chain extensions need in our integration tests
RUN cargo install --git https://github.com/xermicus/substrate-contracts-node.git --branch bn128 --force --locked && \
    rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git" 
