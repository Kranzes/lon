use std::{
    env,
    fmt::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};

use crate::{
    bot::{Forge, GitLab},
    git,
    lock::Lock,
    lon_nix::LonNix,
    sources::{GitHubSource, GitSource, Source, Sources, UpdateSummary},
};

/// The default log level.
///
/// 2 corresponds to the level INFO.
const DEFAULT_LOG_LEVEL: usize = 2;

#[derive(Parser)]
#[command(version)]
pub struct Cli {
    /// Silence all output
    #[arg(short, long)]
    quiet: bool,
    /// Verbose mode (-v, -vv, etc.)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    /// The directory containing lon.{nix,lock}
    #[arg(short, long)]
    directory: Option<PathBuf>,
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize lon.{nix,lock}
    Init,
    /// Add a new source
    Add {
        #[clap(subcommand)]
        commands: AddCommands,
    },
    /// Update an existing source to the newest revision
    Update(UpdateArgs),
    /// Modify an existing source
    ///
    /// When you only change the branch, the newest revision from that branch is locked.
    ///
    /// When you change the revision, the source is locked to this revision.
    Modify(ModifyArgs),
    /// Remove an existing source
    Remove(SourceArgs),
    /// Freeze an existing source
    Freeze(SourceArgs),
    /// Unfreeze an existing source
    Unfreeze(SourceArgs),

    /// Bot that opens PRs for updates
    Bot {
        #[clap(subcommand)]
        commands: BotCommands,
    },
}

#[derive(Subcommand)]
#[clap(rename_all = "lower")]
enum AddCommands {
    /// Add a git source
    ///
    /// It's fetched by checking out the repository.
    Git(AddGitArgs),
    /// Add a github source
    ///
    /// It's fetched as a tarball which is more efficient than checking out the
    /// repository.
    GitHub(AddGitHubArgs),
}

#[derive(Args)]
struct AddGitArgs {
    /// Name of the source
    name: String,
    /// URL to the repository
    url: String,
    /// Branch to track
    branch: String,
    /// Revision to lock
    #[arg(short, long)]
    revision: Option<String>,
    /// Fetch submodules
    #[arg(long)]
    submodules: bool,
    /// Freeze the source
    #[arg(long, default_value_t = false)]
    frozen: bool,
}

#[derive(Args)]
struct AddGitHubArgs {
    /// An identifier made up of {owner}/{repo}, e.g. nixos/nixpkgs
    identifier: String,
    /// Branch to track
    branch: String,
    /// Name of the source
    ///
    /// If you do not supply this, the repository name is used as the source name.
    #[arg(short, long)]
    name: Option<String>,
    /// Revision to lock
    #[arg(short, long)]
    revision: Option<String>,
    /// Freeze the source
    #[arg(long, default_value_t = false)]
    frozen: bool,
}

#[derive(Args)]
struct UpdateArgs {
    /// Name of the source
    ///
    /// If this is omitted, all sources are updated.
    name: Option<String>,
    /// Whether to commit lon.{nix,lock}.
    #[arg(short, long, default_value_t = false)]
    commit: bool,
}

#[derive(Args)]
struct ModifyArgs {
    /// Name of the source
    name: String,
    /// Branch to track
    #[arg(short, long)]
    branch: Option<String>,
    /// Revision to lock
    #[arg(short, long)]
    revision: Option<String>,
}

#[derive(Args)]
struct SourceArgs {
    /// Name of the source
    name: String,
}

#[derive(Subcommand)]
#[clap(rename_all = "lower")]
enum BotCommands {
    /// Run the bot for GitLab
    GitLab,
}

impl Cli {
    pub fn init(module: &str) -> ExitCode {
        let cli = Self::parse();

        let _ = stderrlog::new()
            .module(module)
            .show_level(false)
            .quiet(cli.quiet)
            .verbosity(DEFAULT_LOG_LEVEL + usize::from(cli.verbose))
            .init();

        let directory = match cli.directory {
            Some(directory) => directory,
            None => match std::env::var("LON_DIRECTORY") {
                Ok(dir) => PathBuf::from(dir),
                Err(_) => std::env::current_dir().unwrap_or_default(),
            },
        };

        match cli.commands.call(directory) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                // When at least one -v is added, the source of the error is also printed.
                if DEFAULT_LOG_LEVEL + usize::from(cli.verbose) >= 3 {
                    log::error!("{err:#}");
                } else {
                    log::error!("{err}");
                }
                ExitCode::FAILURE
            }
        }
    }
}

impl Commands {
    pub fn call(self, directory: impl AsRef<Path>) -> Result<()> {
        match self {
            Self::Init => init(directory),
            Self::Add { commands } => match commands {
                AddCommands::Git(args) => add_git(directory, &args),
                AddCommands::GitHub(args) => add_github(directory, &args),
            },
            Self::Update(args) => update(directory, &args),
            Self::Modify(args) => modify(directory, &args),
            Self::Remove(args) => remove(directory, &args),
            Self::Freeze(args) => freeze(directory, &args),
            Self::Unfreeze(args) => unfreeze(directory, &args),

            Self::Bot { commands } => match commands {
                BotCommands::GitLab => bot(directory, &GitLab::from_env()?),
            },
        }
    }
}

fn init(directory: impl AsRef<Path>) -> Result<()> {
    if LonNix::path(&directory).exists() {
        log::info!("lon.nix already exists");
    } else {
        log::info!("Writing lon.nix...");
        LonNix::write(&directory)?;
    }

    if Lock::path(&directory).exists() {
        log::info!("lon.lock already exists");
    } else {
        log::info!("Writing empty lon.lock...");
        let sources = Sources::default();
        sources.write(directory)?;
    }

    Ok(())
}

fn add_git(directory: impl AsRef<Path>, args: &AddGitArgs) -> Result<()> {
    let mut sources = Sources::read(&directory)?;
    if sources.contains(&args.name) {
        bail!("Source {} already exists", args.name);
    }

    log::info!("Adding {}...", args.name);

    let source = GitSource::new(
        &args.url,
        &args.branch,
        args.revision.as_ref(),
        args.submodules,
        args.frozen,
    )?;

    sources.add(&args.name, Source::Git(source));

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    Ok(())
}

fn add_github(directory: impl AsRef<Path>, args: &AddGitHubArgs) -> Result<()> {
    let Some((owner, repo)) = args.identifier.split_once('/') else {
        bail!("Failed to parse identifier {}", args.identifier)
    };

    let name = args.name.clone().unwrap_or(repo.to_string());

    let mut sources = Sources::read(&directory)?;
    if sources.contains(&name) {
        bail!("Source {name} already exists");
    }

    log::info!("Adding {name}...");

    let source = GitHubSource::new(
        owner,
        repo,
        &args.branch,
        args.revision.as_ref(),
        args.frozen,
    )?;

    sources.add(&name, Source::GitHub(source));

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    Ok(())
}

fn update(directory: impl AsRef<Path>, args: &UpdateArgs) -> Result<()> {
    let mut sources = Sources::read(&directory)?;

    let mut names = Vec::new();

    if let Some(ref name) = args.name {
        names.push(name.to_string());
    } else {
        names.extend(sources.names().into_iter().map(ToString::to_string));
    }

    if names.is_empty() {
        bail!("Lock file doesn't contain any sources")
    }

    let mut commit_message = CommitMessage::new();

    for name in &names {
        let Some(source) = sources.get_mut(name) else {
            bail!("Source {name} doesn't exist")
        };

        log::info!("Updating {name}...");

        let summary = source
            .update()
            .with_context(|| format!("Failed to update {name}"))?;

        if let Some(summary) = summary {
            commit_message.add_summary(name, summary);
        }
    }

    if commit_message.is_empty() {
        bail!("No updates available")
    }

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    if args.commit {
        commit(&directory, &commit_message.to_string(), None)?;
    }

    Ok(())
}

fn modify(directory: impl AsRef<Path>, args: &ModifyArgs) -> Result<()> {
    let mut sources = Sources::read(&directory)?;

    let Some(source) = sources.get_mut(&args.name) else {
        bail!("Source {} doesn't exist", args.name)
    };

    log::info!("Modifying {}...", args.name);

    source.modify(args.branch.as_ref(), args.revision.as_ref())?;

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    Ok(())
}

fn remove(directory: impl AsRef<Path>, args: &SourceArgs) -> Result<()> {
    let mut sources = Sources::read(&directory)?;

    if !sources.contains(&args.name) {
        bail!("Source {} doesn't exist", args.name)
    }

    log::info!("Removing {}...", args.name);

    sources.remove(&args.name);

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    Ok(())
}

fn freeze(directory: impl AsRef<Path>, args: &SourceArgs) -> Result<()> {
    let mut sources = Sources::read(&directory)?;

    let Some(source) = sources.get_mut(&args.name) else {
        bail!("Source {} doesn't exist", args.name)
    };

    log::info!("Freezing {}...", args.name);

    source.freeze();

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    Ok(())
}

fn unfreeze(directory: impl AsRef<Path>, args: &SourceArgs) -> Result<()> {
    let mut sources = Sources::read(&directory)?;

    let Some(source) = sources.get_mut(&args.name) else {
        bail!("Source {} doesn't exist", args.name)
    };

    log::info!("Unfreezing {}...", args.name);

    source.unfreeze();

    sources.write(&directory)?;
    LonNix::update(&directory)?;

    Ok(())
}

fn bot(directory: impl AsRef<Path>, forge: &impl Forge) -> Result<()> {
    let base_ref = git::current_rev(&directory)?;

    let result = bot_fallible(&directory, forge, &base_ref);

    // Always return to the base commit.
    git::checkout(&directory, &base_ref, false)?;

    result
}

fn bot_fallible(directory: impl AsRef<Path>, forge: &impl Forge, base_ref: &str) -> Result<()> {
    let mut sources = Sources::read(&directory)?;

    let names = sources
        .names()
        .into_iter()
        .cloned()
        .collect::<Vec<String>>();

    let mut commit_message = CommitMessage::new();

    for name in &names {
        let Some(source) = sources.get_mut(name) else {
            log::warn!("Source {name} doesn't exist");
            continue;
        };

        if source.frozen() {
            log::info!("Source {name} is frozen. Skipping...");
            continue;
        }

        log::debug!("Checking out base ref {base_ref}...");
        git::checkout(&directory, base_ref, false)?;

        let branch = format!("lon/{name}");
        log::debug!("Checking out new branch {branch}...");
        git::checkout(&directory, &branch, true)?;

        log::info!("Updating {name}...");

        let summary = source
            .update()
            .with_context(|| format!("Failed to update {name}"))?;

        let Some(summary) = summary else {
            log::info!("No updates available");
            continue;
        };
        commit_message.add_summary(name, summary);

        sources.write(&directory)?;
        LonNix::update(&directory)?;

        let user_name = env::var("LON_USER_NAME").unwrap_or("LonBot".into());
        let user_email = env::var("LON_USER_EMAIL").unwrap_or("lonbot@lonbot".into());

        log::debug!("Committing changes...");
        commit(
            &directory,
            &commit_message.to_string(),
            Some(git::User::new(&user_name, &user_email)),
        )?;

        let push_url = env::var("LON_PUSH_URL").ok();

        // Never log the URL as it might contain a secret token.
        log::debug!("Force pushing repository...");
        git::force_push(&directory, push_url.as_deref())?;

        let pull_request_url = forge.open_pull_request(&branch, name)?;
        log::info!("Opened Pull Request: {pull_request_url}");
    }

    Ok(())
}

struct CommitMessage(Vec<(String, UpdateSummary)>);

impl CommitMessage {
    fn new() -> Self {
        Self(vec![])
    }

    fn add_summary(&mut self, name: &str, summary: UpdateSummary) {
        self.0.push((name.into(), summary));
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for CommitMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut commit_message = String::new();
        writeln!(&mut commit_message, "lon: update")?;
        writeln!(&mut commit_message)?;
        writeln!(&mut commit_message, "Updated sources:")?;
        writeln!(&mut commit_message)?;

        for (name, summary) in &self.0 {
            writeln!(&mut commit_message, "• {name}:")?;
            writeln!(&mut commit_message, "    {}", summary.old_revision)?;
            writeln!(&mut commit_message, "  → {}", summary.new_revision)?;
        }
        write!(f, "{commit_message}")
    }
}

fn commit(
    directory: impl AsRef<Path>,
    commit_message: &str,
    user: Option<git::User>,
) -> Result<()> {
    git::add(&directory, &[&Lock::path(&directory)])?;
    git::add(&directory, &[&LonNix::path(&directory)])?;
    git::commit(&directory, commit_message, user)?;
    Ok(())
}
