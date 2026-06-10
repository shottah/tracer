# Releasing

Releases are tag-driven and fully automated by
[.github/workflows/release.yml](.github/workflows/release.yml).

1. Update versions: bump `workspace.package.version` in `Cargo.toml` (the
   internal `tracer-*` dependency versions in `[workspace.dependencies]`
   must match), run `cargo build` to refresh `Cargo.lock`.
2. Move the `[Unreleased]` notes in `CHANGELOG.md` under the new version.
3. Commit, then tag and push:

   ```sh
   git tag v0.2.0
   git push origin main --tags
   ```

The workflow builds `tracer` for Linux (x86_64, aarch64), macOS (x86_64,
aarch64), and Windows (x86_64), packages archives with SHA-256 checksums,
and publishes a GitHub release with generated notes.

Crates are not published to crates.io yet; `cargo install --git` is the
supported source install.
