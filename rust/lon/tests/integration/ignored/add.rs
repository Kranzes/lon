use std::fs;

use anyhow::{Result, bail};
use expect_test::expect;
use tempfile::tempdir;

use crate::{init, lon};

#[test]
#[ignore]
fn add_ssh() -> Result<()> {
    add("git@remote:repo.git")
}

fn add(url: &'static str) -> Result<()> {
    let tmpdir = tempdir()?;

    init(tmpdir.path())?;

    let output = lon(tmpdir.path(), ["add", "git", "repo", url, "main"])?;
    if !output.status.success() {
        bail!("Failed to add repo");
    }

    let lock_path = tmpdir.path().join("lon.lock");

    let actual = fs::read_to_string(lock_path)?;
    let expected = expect![[r#"
        {
          "version": "1",
          "sources": {
            "repo": {
              "type": "Git",
              "fetchType": "git",
              "branch": "main",
              "revision": "b6b12ee9cb64f547f129d7d64c104b8d2938dc0f",
              "url": "git@remote:repo.git",
              "hash": "sha256-5wJChh/6lrQodEtR+tPll4Xb6ZzbSF7bGaKwH00toO0=",
              "lastModified": 1745335431,
              "submodules": false
            }
          }
        }
    "#]];
    expected.assert_eq(&actual);

    Ok(())
}
