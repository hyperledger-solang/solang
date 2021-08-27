# Release Checklist

- Update the version in `Cargo.toml`, `docs/conf.py`, and the binary links in `docs/installing.rst`, and `CHANGELOG.md`
- Copy the contents of the CHANGELOG for this release into commit message
- Ensure the cargo publish is happy `cargo publish --dry-run`
- Try the release github actions by pushing a tag to your solang fork
- If build succeeds, merge the release commits
- Apply tag to merged commit on main branch
- Push tag to origin
- Wait for build to succeed
- `cargo publish`
- Release new version of vscode plugin if needed