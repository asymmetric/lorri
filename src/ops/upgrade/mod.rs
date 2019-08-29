//! Upgrade lorri by using nix-env to install from Git.
//!
//! In the future, this upgrade tool should use tagged versions.
//! However, while this repo is closed source, it uses a
//! rolling-release branch.

use crate::changelog;
use crate::cli;
use crate::nix;
use crate::ops::{ExitError, OpResult};
use crate::VERSION_BUILD_REV;
use cas::ContentAddressable;
use std::path::Path;
use std::process::Command;

impl From<cli::UpgradeTo> for String {
    fn from(desc: cli::UpgradeTo) -> Self {
        match desc.source.unwrap_or(cli::UpgradeSource::RollingRelease) {
            cli::UpgradeSource::RollingRelease => String::from("rolling-release"),
            cli::UpgradeSource::Master => String::from("master"),
            cli::UpgradeSource::Local(src) => String::from(
                src.path
                    .to_str()
                    .expect("Requested Lorri source directory not UTF-8 clean"),
            ),
        }
    }
}

fn upgrade_callopts<'a>(upgrade_expr: &'a Path, upgrade_target: &str) -> nix::CallOpts<'a> {
    println!("Upgrading from source: {}", upgrade_target);
    let mut expr = nix::CallOpts::file(&upgrade_expr);
    expr.argstr("src", upgrade_target);
    expr
}

/// nix-env upgrade Lorri in the default profile.
pub fn main(upgrade_target: cli::UpgradeTo, cas: &ContentAddressable) -> OpResult {
    /*
    1. nix-instantiate the expression
    2. get all the changelog entries from <currentnumber> to <maxnumber>
    3. nix-build the expression's package attribute
    4. nix-env -i the package
     */
    let upgrade_expr = cas
        .file_from_string(include_str!("./upgrade.nix"))
        .expect("could not write to CAS");
    let upgrade_target = String::from(upgrade_target);

    let mut expr = upgrade_callopts(&upgrade_expr, &upgrade_target);
    let changelog: changelog::Log = expr.attribute("changelog").value().unwrap();

    println!("Changelog when upgrading from {}:", VERSION_BUILD_REV);
    for entry in changelog.entries {
        if VERSION_BUILD_REV < entry.version {
            println!();
            println!("{}:", entry.version);
            for line in entry.changes.lines() {
                println!("    {}", line);
            }
        }
    }

    let mut expr = upgrade_callopts(&upgrade_expr, &upgrade_target);
    println!("Building ...");
    match expr.attribute("package").path() {
        Ok((build_result, gc_root)) => {
            let status = Command::new("nix-env")
                .arg("--install")
                .arg(build_result.as_path())
                .status()
                .expect("Error: failed to execute nix-env --install");
            // we can drop the temporary gc root
            drop(gc_root);

            if status.success() {
                Ok(Some(String::from("\nUpgrade successful.")))
            } else {
                Err(ExitError::errmsg(String::from(
                    "\nError: nix-env command was not successful!",
                )))
            }
        }
        Err(e) => Err(ExitError::errmsg(format!(
            "Failed to build the update! Please report a bug!\n\
             {:?}",
            e
        ))),
    }
}
