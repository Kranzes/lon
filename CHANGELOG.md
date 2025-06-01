# Changelog

## 0.7.0 (unreleased)

- Renamed parameters for `lon init`: `--type` -> `--from` & `--from` ->
  `--source`.

## 0.6.0

- Fixed a redundant download when prefetching and then using a git source.
- Added the ability to initalize from a Niv `sources.json`. This re-locks the
  listed revs. It guarantees that the revs don't change. The hashes however
  might change.

## 0.5.0

- Fixed a redundant download when prefetching and then using a tarball.
- Added Forgejo support to the bot
- Added the ability to include the list of commits between updates in the bot.
  This an be configured via the environment variable `LON_LIST_COMMITS`. You
  can either set this to `true` or an integer. This configures the number of
  commits to list. The default is 50.

## 0.4.0

- Fixed fetching submodules.
- Added the ability to read the directory in which to look for Lon's files from
  the environment variable `LON_DIRECTORY`.
- Added the subcommand `bot` to create a PR for each source that can be
  updated. Currently GitLab and GitHub are supported. This is meant to run
  inside a forge native CI (GitLab CI/CD or GitHub Actions) on a schedule.

## 0.3.0

- Added a `--version` and `-V` flag to display the version of Lon.
- Added the ability to freeze sources so that they are not updated via `lon
  update`. Sources can be frozen when they're added by providing the flag
  `--frozen` or they can be frozen or unfrozen later by calling `lon freeze`
  and `lon unfreeze` respectively.
- Fixed caching of Git sources in Nix Store by including `lastModified` in
  lon.lock.
