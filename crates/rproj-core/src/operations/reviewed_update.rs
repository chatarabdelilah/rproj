use super::*;
#[cfg(windows)]
use crate::creation::protocol::{read_json, write_atomic};
use crate::updates::review::*;

impl WorkbenchClient {
    pub fn reviewed_update_support(&self) -> Result<()> {
        let version = query(
            &self.worker,
            &[WORKER_FLAG, "reviewed-update-support"],
            None,
        )
        .context("Apply requires a matching desktop and CLI distribution")?;
        ensure!(
            version.trim().parse::<u32>()? == PROTOCOL,
            "Unsupported update worker"
        );
        Ok(())
    }
    pub fn reviewed_update(
        &self,
        action: Action,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<State> {
        #[cfg(not(windows))]
        {
            let _ = (action, sink);
            bail!("Reviewed updates currently require the Windows worker");
        }
        #[cfg(windows)]
        {
            let _guard = self
                .busy
                .try_lock()
                .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
            let dir = tempfile::Builder::new()
                .prefix("rproj-reviewed-update-")
                .tempdir()?;
            let request = Request {
                protocol: PROTOCOL,
                id: dir
                    .path()
                    .file_name()
                    .context("Missing operation identity")?
                    .to_string_lossy()
                    .into_owned(),
                action,
            };
            write_atomic(&dir.path().join("request.json"), &request)?;
            sink(OperationEvent::Started);
            let exit = execute_owned_os(
                &self.worker,
                &[
                    WORKER_FLAG.into(),
                    "reviewed-update-job".into(),
                    dir.path().as_os_str().into(),
                ],
                dir.path(),
                &Cancellation::default(),
                Arc::new(sink),
            )?;
            let report = read_json::<Report>(&dir.path().join("report.json")).and_then(|r| {
                r.verify(&request, exit == OperationOutcome::Succeeded)?;
                Ok(r)
            });
            match report {
                Ok(r) => Ok(r.state),
                Err(e) => Ok(State::Uncertain(format!(
                    "Completion unconfirmed: {e:#}. Inspect project files and check for updates again."
                ))),
            }
        }
    }
}
