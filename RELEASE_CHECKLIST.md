# Release Checklist

- Update the version in `Cargo.toml`, `solang-parser/Cargo.toml`, the binary
  links in `docs/installing.rst`, and `CHANGELOG.md`. Remember to match the
  solang-parser version in the top level `Cargo.toml`.
- Copy the contents of the CHANGELOG for this release into commit message,
  using `git commit -s --cleanup=whitespace` so the that the lines beginning
  with `#` are not removed.
- Try the release github actions by pushing a tag to your solang fork
- Ensure the release text uses the markdown formatting
- If build succeeds, merge the release commits
- Open a PR on Solang's repository containing the release changes, and wait for approval
- Run `cargo publish --dry-run` in the `solang-parser` folder.
- Publish the solang-parser crate, by running `cargo publish` in the solang-parser folder
- Ensure the cargo publish is happy `cargo publish --dry-run`
- Merge the PR
- Apply tag to merged commit on main branch
- Push tag to origin
- Wait for build to succeed
- `cargo publish`
- Release new version of vscode plugin if needed
- Mention release in Discord (Solana, Hyperledger) and Hyperledger /dev/weekly
- Update the version number and the MacOS binaries' sha256 hash in `Casks/solang.rb` under
  the repository `hyperledger/homebrew-solang`.
