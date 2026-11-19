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

## November 19, 2026 verification

The three cached crate archives match their SHA-256 checksums in Cargo.lock.
The finished Windows ZIP now has two reviewed omissions across 590 packages.

| Crate | Published archive SHA-256 | Evidence and packaged disposition |
| --- | --- | --- |
| mac 0.1.1 | `c41e0c4fef86961ac6d6f8a82609f55f31b05e4fce149ac5710e439df7619ba4` | This older archive omits Git metadata. All 12 file paths and blob hashes match the [upstream release tree](https://github.com/reem/rust-mac/tree/66afc663b68a65633ea165c742b5a9c6734581c2), still the default branch head. The [license request](https://github.com/reem/rust-mac/issues/14) remains open. Preserve the manifest and README and report the omission. |
| seahash 4.1.0 | `1c107b6f4780854c8b126e228ea8869f4d7b71260f962fefb57b996b8959ba6b` | The archive records Git revision `94b632a`. A fresh fetch from the canonical GitLab repository confirmed that revision is the first parent of [upstream merge !8](https://gitlab.redox-os.org/redox-os/seahash/-/commit/74c02182146d1edd7c91a9e6eddefc7390682a70). Its only tree change adds LICENSE. Preserve that later file verbatim as a supplement. [Mirror of the same merge](https://github.com/aleksa2808/seahash/commit/74c02182146d1edd7c91a9e6eddefc7390682a70). |
| windows-permissions 0.2.4 | `9e2ccdc3c6bf4d4a094e031b63fadd08d8e42abd259940eb8aa5fdc09d4bf9be` | The archive identifies the [release tree](https://github.com/danieldulaney/windows-permissions-rs/tree/8740e4efbd88dd01046ad9c169894f3a52eb6e2c), still the default branch head. Neither that tree nor the reviewed upstream issue/PR history supplied a notice for this version. Preserve the manifest and README and report the omission. |

The verifier reads the finished ZIP, checks each recorded notice hash, checks the two
fallbacks, and compares the omissions in dependencies.json and THIRD-PARTY.md.
Retire an exception when its exact version leaves the graph or gains a verified
notice. A new omission requires an explicit review of the corresponding release.

When dependencies change, re-inspect the selected graph and upstream release
identity before adding supplements. Do not copy a newer license onto an older
release without recording that distinction. Packaging verifies supplement
hashes and does not fetch notices from the network.
