use std::{
    fs::{self, create_dir_all, remove_dir_all, OpenOptions, Permissions, set_permissions},
    io::Write,
    path::{Path, PathBuf},
    process::Command, env::{set_current_dir, current_dir}, os::unix::fs::PermissionsExt as _,
};

use anyhow::{Context, Ok, Result};
use clap::Args;
use flake_ctl_build::{copy_configs, export_flake, Builder, SetupInfo, BuilderInfo, CompileInfo};
use flakes::{config::{itf::FlakeConfig, FLAKE_DIR}, paths::{RootedPath, PathExt}};
use fs_extra::{file::write_all, copy_items, dir::{CopyOptions, self}};
use tempfile::tempdir;

fn main() -> Result<()> {
    flake_ctl_build::run::<DPKGBuilder>()
}

struct DPKGBuilder;

impl Builder for DPKGBuilder {
    // fn description(-> &str {
    //     "Package flakes with dpkg-deb"
    // }

    fn setup(location: &Path, info: SetupInfo) -> Result<BuilderInfo> {
        infrastructure(location, create_dir_all)?;

        control(&info.options, location, info.config.engine().pilot(), false).context("Failed to create control file")?;
        rules(location).context("Failed to create rules file")?;
        postinst(location, &info.config).context("Failed to create install script")?;
        prerm(location, &info.config).context("Failed to create uninstall script")?;
        compat(location)?;
        source_format(location)?;
        install(location, &info.flake_name, &info.config)?;
        changelog(location, &info.options)?;

        let export_path = &location.join(flakes::config::FLAKE_DIR.strip_prefix("/").unwrap());
        create_dir_all(export_path)?;
        create_dir_all(location.join("tmp"))?;

        Ok(BuilderInfo {
            image_location: location.join_ignore_abs("tmp"),
            config_location: location.join_ignore_abs(FLAKE_DIR.as_path()),
        })
    }

    fn compile(location: &Path, info: CompileInfo, target: &Path) -> Result<()> {
        // Move all content of staging directory to a temporary folder, cd there, build, and then copy the result to the output folder
        // This is needed because of a quirk in dpkg-buildpackage where the package is always build in `..`
        let tmp = tempdir()?;
        copy_items(&[location], tmp.path(), &CopyOptions::new())?;
        let cwd = current_dir()?;
        let tmp_build = tmp.path().join(location.file_name().unwrap());
        set_current_dir(&tmp_build)?;

        Command::new("dpkg-buildpackage").args(["-us", "-uc"]).status()?;

        set_current_dir(cwd)?;
        remove_dir_all(tmp_build)?;
        dir::copy(tmp.path(), target.unwrap_or(Path::new(".")), &CopyOptions::new().content_only(true).overwrite(true))?;
        Ok(())
    }
}

    fn infrastructure<F, P>(location: P, f: F) -> Result<()>
    where
        F: FnMut(PathBuf) -> Result<(), std::io::Error>,
        P: AsRef<Path>,
    {
        ["tmp", "usr", "debian/source"].into_iter().map(|x| location.as_ref().join(x)).try_for_each(f)?;
        Ok(())
    }

    fn control(options: &PackageOptions, location: &Path, pilot: &str, edit: bool) -> Result<()> {
        let depends = fs::read_to_string(template.join(pilot))?;

        let mut cfile = OpenOptions::new().truncate(true).create(true).write(true).open(location.join("debian").join("control"))?;


        [
            // Do not remove as_str() or the type inference will break
            ("Source", options.name.as_str()),
            ("Section", "other"),
            ("Priority", "optional"),
            ("Maintainer", &format!("\"{}\" <{}>", options.maintainer_name, options.maintainer_email)),
            ("Homepage", &options.url),
            // TODO: This is just the newest version for now
            ("Standards-Version", "4.6.2.0"),
            ("Rules-Requires-Root", "binary-targets"),
        ]
        .into_iter()
        .try_for_each(|(name, value)| cfile.write_all(format!("{name}: {value}\n").as_bytes()))?;

        cfile.write_all("\n\n".as_bytes())?;

        [
            // Do not remove as_str() or the type inference will break
            ("Package", options.name.as_str()),
            ("Architecture", "all"),
            ("Version", &options.version),
            ("Depends", &depends),
            ("Multi-Arch", "foreign"),
            ("Description", &options.description),
            ("Package-Type", "deb"),
        ]
        .into_iter()
        .try_for_each(|(name, value)| cfile.write_all(format!("{name}: {value}\n").as_bytes()))?;


        // TODO: Select default text editor
        if edit {
            Command::new("vi").arg(location.join("debian").join("control")).status().context("Failed to open text editor")?;
        }
        Ok(())
    }

    fn rules(location: &Path) -> Result<()> {
        OpenOptions::new().create(true).write(true).open(location.join("debian/rules"))?.write_all("#!/usr/bin/make -f\n%:\n\tdh $@".as_bytes())?;
        set_permissions(location.join("debian/rules"), Permissions::from_mode(0o777))?;
        Ok(())
    }

    fn postinst(location: &Path, conf: &FlakeConfig) -> Result<()> {
        let pilot = conf.engine().pilot();
        let mut script = OpenOptions::new().create(true).write(true).open(location.join("debian").join("postinst"))?;

        let archive = Path::new("/tmp").join(conf.runtime().image_name());
        let archive = archive.to_string_lossy();
        // TODO: Needs to be read from template for other pilots
        script.write_all(format!("podman load < {archive} && rm {archive}\n").as_bytes())?;

        if let Some((first, mut rest)) = conf.runtime().get_symlinks() {
            let first = first.to_string_lossy();
            script.write_all(format!("ln -s /usr/bin/{pilot}-pilot {first}\n").as_bytes())?;
            rest.try_for_each(|path| script.write_all(format!("ln -s {first} {}\n", path.to_string_lossy()).as_bytes()))?;
        }
        Ok(())
    }

    fn prerm(location: &Path, conf: &FlakeConfig) -> Result<()> {
        let mut script = OpenOptions::new().create(true).write(true).open(location.join("debian").join("prerm"))?;
        script.write_all(format!("podman rmi {}\n", conf.runtime().image_name()).as_bytes())?;
        conf.runtime()
            .paths()
            .keys()
            .map(|x| x.to_string_lossy())
            .try_for_each(|path| script.write_all(format!("rm {path}\n").as_bytes()))?;
        Ok(())
    }

    fn compat(location: &Path) -> Result<()> {
        write_all(location.join("debian/compat"), "10\n")?;
        Ok(())
    }
    
    fn changelog(location: &Path, options: &PackageOptions) -> Result<()> {
        let PackageOptions { name, version, maintainer_name, maintainer_email, .. } = options;
        // TODO: make settable in options
        let status = "UNRELEASED";
        let urgency = "medium";
        // TODO: Maybe offer feature extract from other package
        let content = "* This is an automatically packaged flake, see chengelog of original for details";
        
        let time = chrono::Local::now().format("%a, %d %b %Y %H:%M:%S %z");

        let content = format!("{name} ({version}) {status}; urgency={urgency}\n\n  {content}\n\n -- {maintainer_name} <{maintainer_email}>  {time}\n");
        write_all(location.join("debian/changelog"), &content)?;
        Ok(())
    }

    fn install(location: &Path, flake_name: &str, config: &FlakeConfig) -> Result<()> {
        let yaml = Path::new("usr/share/flakes").join(flake_name).with_extension("yaml");
        let yaml = yaml.to_string_lossy();
        let d = Path::new("usr/share/flakes").join(flake_name).with_extension("d");
        let d = d.to_string_lossy();
        let image = Path::new("tmp").join(config.runtime().image_name());
        let image = image.to_string_lossy();
        write_all(location.join("debian/install"), &format!("{yaml} /usr/share/flakes\n{d} /usr/share/flakes\n{image} /tmp\n"))?;
        Ok(())
    }

    fn source_format(location: &Path) -> Result<()> {
        write_all(location.join("debian/source/format"), "1.0\n")?;
        Ok(())
    }
