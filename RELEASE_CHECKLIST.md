# Release Checklist

- Update the version in `Cargo.toml`, `solang-parser/Cargo.toml`, the binary
  links in `docs/installing.rst`, and `CHANGELOG.md`. Remember to match the
  solang-parser version in the top level `Cargo.toml`.
- Copy the contents of the CHANGELOG for this release into commit message,
  using `git commit -s --cleanup=whitespace` so the that the lines beginning
  with `#` are not removed.
- Ensure the cargo publish is happy `cargo publish --dry-run`
- Try the release github actions by pushing a tag to your solang fork
- Ensure the release text uses the markdown formatting
- If build succeeds, merge the release commits
- Apply tag to merged commit on main branch
- Push tag to origin
- Wait for build to succeed
- `cargo publish`
- Release new version of vscode plugin if needed
- Mention release in Discord (Solana, Hyperledger) and Hyperledger /dev/weekly
