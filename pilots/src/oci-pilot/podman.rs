use crate::prunner::PodmanRunner;
use std::{io::Error, path::PathBuf};

/// Podman runtime
///
pub(crate) struct PodmanPilot {
    appdir: PathBuf,
    runner: PodmanRunner,
    debug: bool,
}

impl PodmanPilot {
    /// Constructor of a new Podman Pilot instance
    pub(crate) fn new(debug: bool) -> Result<Self, Error> {
        let appdir = flakes::config::app_path()?;
        Ok(PodmanPilot {
            appdir: appdir.to_owned(),
            runner: PodmanRunner::new(appdir.file_name().unwrap().to_str().unwrap().to_string(), flakes::config::get(), debug),
            debug,
        })
    }

    /// Start Podman Pilot instance
    pub(crate) fn start(&mut self) -> Result<(), Error> {
        let (stdout, stderr) = self.runner.start()?;
        if !stdout.is_empty() {
            println!("{}", stdout);
        }

        if !stderr.is_empty() {
            log::error!("{}", stderr);
        }

        Ok(())
    }

    /// Returns true if a container is running
    fn is_running(&self) -> Result<bool, Error> {
        Ok(false)
    }

    fn is_image_exists(&self, name: &str) -> Result<bool, Error> {
        Ok(false)
    }

    /// Find container by the ID and call an action there
    fn call_instance(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Get relevant exported path
    fn get_exported_path(&self) -> PathBuf {
        PathBuf::from("")
    }

    /// Mount container. Returns the mount point, if succeeded
    fn mount(&self, as_image: bool) -> Result<PathBuf, Error> {
        Ok(PathBuf::from("/mount/point"))
    }

    /// Umount container.
    fn umount(&self, mountpoint: PathBuf, as_image: bool) -> Result<(), Error> {
        Ok(())
    }
}
