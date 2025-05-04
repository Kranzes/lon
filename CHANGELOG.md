# Changelog

## 0.4.0 (unreleased)

- Fixed fetching submodules.
- Added the ability to read the directory in which to look for Lon's files from
  the environment variable `LON_DIRECTORY`.
- Added the subcommand `bot` to create a PR for each source that can be
  updated. This is meant to run inside CI on a schedule. Currently, only GitLab
  is supported.

## 0.3.0

- Added a `--version` and `-V` flag to display the version of Lon.
- Added the ability to freeze sources so that they are not updated via `lon
  update`. Sources can be frozen when they're added by providing the flag
  `--frozen` or they can be frozen or unfrozen later by calling `lon freeze`
  and `lon unfreeze` respectively.
- Fixed caching of Git sources in Nix Store by including `lastModified` in
  lon.lock.
