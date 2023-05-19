// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]

mod tests {
    use crate::Cli;
    use clap::CommandFactory;

    #[test]
    fn test() {
        Cli::command().debug_assert();
    }
}
