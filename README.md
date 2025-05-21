# Lon

Lock & update Nix dependencies.

## Features

- Only uses SRI hashes
- Supports fixed outputs of `builtins.fetchGit` by using an SRI hash and thus
  enables caching for these sources in the Nix Store
- Allows overriding dependencies via an environment variable for local
  development
- Leverages modern Nix features (concretely this means Nix >= 2.4 is required)
- Built-in bot to automate dependency updates for GitHub, GitLab, and Forgejo

## Installation

The easiest way to use Lon is directly from Nixpkgs. It is currently available
in the `nixos-unstable` branch and will be included in NixOS releases starting
from 25.05.

You can also invoke it via `nix run github:nikstur/lon`.

```console
$ lon
Usage: lon [OPTIONS] <COMMAND>

Commands:
  init      Initialize lon.{nix,lock}
  add       Add a new source
  update    Update an existing source to the newest revision
  modify    Modify an existing source
  remove    Remove an existing source
  freeze    Freeze an existing source
  unfreeze  Unfreeze an existing source
  bot       Bot that opens PRs for updates
  help      Print this message or the help of the given subcommand(s)

Options:
  -q, --quiet                  Silence all output
  -v, --verbose...             Verbose mode (-v, -vv, etc.)
  -d, --directory <DIRECTORY>  The directory containing lon.{nix,lock}
  -h, --help                   Print help
  -V, --version                Print version
```

## Usage

Initialize Lon:

```console
$ lon init
Writing lon.nix...
Writing empty lon.lock...
```

Add a new GitHub source:

```console
$ lon add github nixos/nixpkgs master
Adding nixpkgs...
Locked revision: 543931cdbf2b2313479c391d956edb5347362744
Locked hash: sha256-8pTC0OIYD47alDVf2mwSytwARCwoH6IqnUfpyshyQX8=
```

Add a new Git source:

```console
$ lon add git lix https://git.lix.systems/lix-project/lix.git main
Adding lix...
Locked revision: a510d1748416ff29b1ed3cab92ac0ad943b6e590
Locked hash: sha256-IjSu5PnS+LFqHfJgueDXrqSBd9/j9GxAbrFK8F1/Z5Y=
Locked lastModified: 1724864109
```

Git sources also support fetching submodules. Enable it by supplying
`--submodules` to Lon.

You can now access these sources via `lon.nix`:

```nix
let
  sources = import ./lon.nix;
  pkgs = import sources.nixpkgs { };
  lix = import sources.lix;
in
  {
    nix = pkgs.nix;
    lix = lix.packages.x86_64-linux.default;
  }
```

You can update individual sources via `lon update nixpkgs` or all sources via
`lon update`. You can even let Lon create a commit for the updates it performs
via `lon update --commit`. The commit message will list all the updates
performed similar to the way `nix flake update --commit-lock-file` does.

### Overriding a Source for Local Development

You can use environment variables that follow the scheme `LON_OVERRIDE_${name}`
to override a source for local development. Lon will use the path this variable
points to instead of the fetching the locked source from `lon.lock`.

Note that no sanitizing of names is performed by Lon. That's why you should
give your sources names that only contain alphanumeric names.

## Bot

With the subcommand `bot <forge>`, you can automatically update your sources. Lon
iterates over each source and if an update is available, performs it and opens
a PR.

Currently, GitLab (`gitlab`), GitHub (`github`) and Forgejo (`forgejo`) are supported.

```console
Bot that opens PRs for updates

Usage: lon bot <COMMAND>

Commands:
  gitlab   Run the bot for GitLab
  github   Run the bot for GitHub
  forgejo  Run the bot for Forgejo
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### GitLab Usage

1. Create a [Project Access Token] with the role `Developer`, and the `api` and
   `write_repository` scope. You can also create a [Group Access Token] so that
   the entire group can use the bot.
2. Store the token in a CI/CD variable called `PROJECT_ACCESS_TOKEN`.
3. Configure a [Scheduled Pipeline].
4. Extend your `.gitlab-ci.yml` with the following snippet. Make sure to set
   `LON_PUSH_URL` including the token stored in `PROJECT_ACCESS_TOKEN`.

```yml
stages:
  - update

lon:
  stage: update
  rules:
    # Only run on a schedule and only on the main branch.
    - if: $CI_PIPELINE_SOURCE == "schedule" && $CI_COMMIT_REF_NAME == $CI_DEFAULT_BRANCH
  variables:
    LON_TOKEN: "$PROJECT_ACCESS_TOKEN"
    LON_PUSH_URL: "https://token:${LON_TOKEN}@${CI_SERVER_HOST}/${CI_PROJECT_PATH}.git"
    LON_LABELS: "bot,lon"
  script:
    - lon bot gitlab
```

[Project Access Token]: https://docs.gitlab.com/user/project/settings/project_access_tokens/
[Group Access Token]: https://docs.gitlab.com/user/group/settings/group_access_tokens/
[Scheduled Pipeline]: https://docs.gitlab.com/ci/pipelines/schedules/

### GitHub Usage

1. [Allow GitHub Actions to create Pull
   Requests](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository#preventing-github-actions-from-creating-or-approving-pull-requests)
2. Add a workflow for updates (e.g. `.github/workflows/update.yml`). Use the
   following snippet to create a functioning workflow. Note specifically the
   permissions and environment variables.

```yml
jobs:
  update:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
      issues: write
    steps:
      - uses: actions/checkout@v4
      - env:
          LON_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          LON_LABELS: "lon,bot"
        run: lon bot github
```

### Forgejo Usage

#### Basic usage

Add a workflow for updates (e.g. `.forgejo/workflows/update.yml`). Use the
following snippet to create a functioning workflow.

```yml
jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - env:
          LON_TOKEN: ${{ secrets.FORGEJO_TOKEN }}
          LON_LABELS: "lon,bot"
        run: lon bot forgejo
```

Note, however, that the pull requests opened via this actions will not trigger workflows
due to how the the [automatic token](https://forgejo.org/docs/latest/user/actions/#automatic-token) is designed.

#### With an Access Token

To alleviate the previous problem, it is possible to create a personal access token to
use instead of the automatic one.

1. Create an [Access Token] with the `write:repository` scope
2. Add the token to the actions secret variables

The next snippet creates such a workflow.

```yml
jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          token: ${{ secrets.ACCESS_TOKEN }}
      - env:
          LON_TOKEN: ${{ secrets.ACCESS_TOKEN }}
          LON_LABELS: "lon,bot"
        run: lon bot forgejo
```

[Access Token]: https://docs.codeberg.org/advanced/access-token/

### Config

The bot is configured exclusively via environment variables.

#### Required

- `LON_TOKEN`: The token to access the forge API and push to the repository.

#### Optional

- `LON_USER_NAME`: The Git user name under which the changes are made.
- `LON_USER_EMAIL`: The Git user email under which the changes are made.
- `LON_LABELS`: The labels to set on the Pull Request as a comma separated
  string (e.g. `"lon,bot"`).
- `LON_PUSH_URL`: The URL to use to push to the repository. This can be used to
  set a token in the URL. For GitLab, this is required.
- `LON_LIST_COMMITS`: The number of commits to list in the commit message that
  occured between the old revision and the updated revision. If this is unset,
  none are listed.

#### GitLab Specific (Required)

These are [predefined in GitLab
CI/CD](https://docs.gitlab.com/ci/variables/predefined_variables/#predefined-variables).

- `CI_API_V4_URL`
- `CI_PROJECT_ID`
- `CI_DEFAULT_BRANCH`

#### GitHub Specific (Required)

These are [predefined in GitHub
Actions](https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/store-information-in-variables#default-environment-variables).

- `GITHUB_REPOSITORY`

## Contributing

Contributions are welcome!

### Tests

Lon has a growing test suite that consists of two parts:

- normal Rust unit/integration tests
- VM tests

The VM tests are also written in Rust but are ignored when you call `cargo
test`. They are designed to only run inside a VM because they access resources
mocked by another VM. You can call these VM tests via `nix build
.#checks.x86_64-linux.lon`.

You can add another VM test by creating one in inside the `ignored` module of
the Rust integration tests.

All the tests are included in the flake checks. You can run all of them via
`nix flake check`.

### Invariants

- Support only few repository hosters: Lon does not aim to support all possible
  repository hosters. It will focus on the most important ones and will as much
  as possible rely on generic protocols (e.g. Git) to find and lock updates.
  GitHub is already an exception to this rule, but because of its ubiquity and
  importance, it is unavoidable.
- No tracking besides Git branches. You can still lock e.g. a specific
  revision, but you will have to update it manually.

## On the Shoulders of Giants

Lon is heavily inspired by [niv](https://github.com/nmattia/niv) and
[npins](https://github.com/andir/npins) and builds on their success.
