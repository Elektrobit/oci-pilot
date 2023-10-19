use std::{fs, io::Error, path::PathBuf};

use crate::pdsys::PdSysCall;

/// Garbage Collector.
///
/// When running Podman, CID file is created. What happens next with that file,
/// it all depends on a use case. They are removed after the cycle, if a flake
/// is to be recycled, but for whatever reasons if a bogus CID file exists,
/// Pilot first should check if it is valid, and this takes extra time.

#[derive(Clone)]
pub struct CidGarbageCollector {
    debug: bool,
    pds: PdSysCall,
}

impl CidGarbageCollector {
    /// Create an instance of a CidGarbageCollector class
    pub fn new(debug: bool) -> Self {
        CidGarbageCollector { pds: PdSysCall::new(debug), debug }
    }

    /// Check if a given CID is valid
    pub fn on_cidfile(&self, cidfile: PathBuf) -> Result<(bool, String), Error> {
        if !cidfile.exists() {
            if self.debug {
                log::debug!("No CID file: {:?}", cidfile);
            }
            return Ok((false, "".to_string()));
        }

        let cid = &fs::read_to_string(&cidfile)?;

        match self.pds.call(false, &["container", "exists", cid.trim()]) {
            Ok(_) => {
                if self.debug {
                    log::debug!("Container with CID {:?} exists", cidfile);
                }
                Ok((true, cid.to_string()))
            }
            Err(_) => {
                fs::remove_file(&cidfile)?;

                if self.debug {
                    log::debug!("Container with CID {:?} does not exist, removing CID", cidfile);
                }

                Ok((false, "".to_string()))
            }
        }
    }

    /// Check all existing CID files for their validity
    pub fn on_all(&self) -> Result<(), Error> {
        log::debug!("GC start");
        for e in flakes::config::get_cid_store()?.read_dir()? {
            if let Ok(e) = e {
                match self.on_cidfile(e.path()) {
                    Ok(r) => {
                        if !r.0 {
                            log::debug!("Removed {:?}", e.file_name());
                        }
                    }
                    Err(err) => {
                        log::error!("Garbage collector error: {}", err);
                    }
                }
            }
        }
        log::debug!("GC finished");
        Ok(())
    }

    /// Remove CID file no matter what. Used usually as a deferred method
    /// on non-resumable Flake
    pub fn force_cidfile(&self, cidfile: PathBuf) -> Result<(), Error> {
        Ok(())
    }
}
