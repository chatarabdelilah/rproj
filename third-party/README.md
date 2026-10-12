# Desktop dependency notice supplements

The portable archive generator copies actual license and notice files from the
locked Windows dependency graph. Some published crates omit the license files
kept at their upstream workspace root. `supplements.json` identifies the exact
crate versions, published Git commits from `.cargo_vcs_info.json`, immutable
upstream URLs and SHA-256 hashes of the added text under `licenses/`. A later
upstream notice correction has a separate `notice_commit` and explanation;
the generated inventory preserves both commits in `supplement_provenance`.

These are verbatim upstream files, not replacement licenses. GPUI Kit's Apache
notice is also included for its assets package alongside the published Lucide
notice. No custom fonts or Charmed Icons are bundled in this alpha.

`simd_helpers` 0.1.0 is supplemented from [upstream's license-only correction](https://github.com/lu-zero/simd_helpers/commit/82040194cd05affb060bf94d6f19f82a771d07fb).
The published commit `ca1a2f84aa386d758e98f8a609d990263932fb85` is its sole
parent; that correction adds only `LICENSE`, preserving all code and the MIT
declaration. The notice was added after publication; it is not presented as a
file from the published archive.

Three legacy distributions still contain declarations without standalone notice
text, checked again October 12, 2026:

- `mac` 0.1.1 has no notice text in [the upstream tree](https://github.com/reem/rust-mac/tree/66afc663b68a65633ea165c742b5a9c6734581c2).
  Its [request for a license file](https://github.com/reem/rust-mac/issues/14)
  remains open without a maintainer-provided notice.
- `seahash` 4.1.0's original GitLab raw/API endpoints returned HTTP 403. Its
  published Git object `94b632aeac099031c373599313d5b5f0acbbaec0` is preserved
  by [one mirror](https://github.com/aleksa2808/seahash/tree/94b632aeac099031c373599313d5b5f0acbbaec0)
  and [a second mirror](https://github.com/planet-s/seahash/tree/94b632aeac099031c373599313d5b5f0acbbaec0);
  both trees contain no notice. A later fork-added text is not used as evidence
  of an original-author notice. An original-author notice also exists in
  [TFS](https://github.com/redox-os/tfs/blob/master/LICENSE), but its adjacent
  [SeaHash manifest](https://github.com/redox-os/tfs/blob/master/seahash/Cargo.toml)
  identifies version 3.0.5; it is not recorded as a notice for 4.1.0.
- `windows-permissions` 0.2.4 has no notice file in [its published upstream tree](https://github.com/danieldulaney/windows-permissions-rs/tree/8740e4efbd88dd01046ad9c169894f3a52eb6e2c)
  or the current repository root; upstream history has no `LICENSE` addition.
  Published source headers, upstream issue/PR discussions, and release records
  also yielded no original notice in the October 12 check.

Packaging preserves the available manifest/README attribution and
lists these gaps explicitly in `THIRD-PARTY.md` and `dependencies.json`.
Distribution notice completeness remains unverified for these omissions.

When dependencies change, re-inspect the selected graph and upstream release
identity before adding supplements. Do not copy a newer license onto an older
release without recording that distinction. Packaging verifies supplement
hashes and does not fetch notices from the network.
