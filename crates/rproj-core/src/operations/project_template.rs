use super::*;
use crate::project_template::{protocol::*, review::*};

impl WorkbenchClient {
    pub fn project_template_support(&self) -> Result<()> {
        let text = query(&self.worker, &[WORKER_FLAG, "template-support"], None)
            .context("Project Template is unavailable; use a matching desktop/CLI distribution")?;
        ensure!(
            serde_json::from_str::<u32>(&text)? == TEMPLATE_PROTOCOL,
            "Unsupported Project Template worker; use a matching distribution"
        );
        Ok(())
    }

    pub fn project_template_context(
        &self,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<LoadedTemplate> {
        successful(self.template_operation(TemplateJob::Context, sink)?)?
            .loaded
            .context("Missing template context")
    }

    pub fn prepare_project_template(
        &self,
        source: TemplateSource,
        action: TemplateAction,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<PreparedTemplate> {
        successful(self.template_operation(TemplateJob::Prepare { source, action }, sink)?)?
            .prepared
            .context("Missing template review")
    }

    /// Validation, Save and Reset are noncancellable; closing waits for owned exit.
    pub fn apply_project_template(
        &self,
        prepared: PreparedTemplate,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<TemplateOutcome> {
        let path = prepared.review.source.path.clone();
        let report = match self.template_operation(TemplateJob::Execute(Box::new(prepared)), sink) {
            Ok(report) => report,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            Err(error) => {
                return Ok(TemplateOutcome {
                    path,
                    status: TemplateStatus::Failed,
                    mutation: TemplateMutation::NotStarted,
                    saved: None,
                    message: Some(format!(
                        "Template operation could not start; no writes were made: {error:#}"
                    )),
                });
            }
        };
        Ok(TemplateOutcome {
            path,
            status: report.status.unwrap_or(TemplateStatus::Uncertain),
            mutation: report.mutation,
            saved: report.saved,
            message: report.message,
        })
    }

    fn template_operation(
        &self,
        action: TemplateJob,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<TemplateReport> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new()
            .prefix("rproj-template-")
            .tempdir()?;
        let request = TemplateRequest {
            protocol: TEMPLATE_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing operation identity")?
                .to_string_lossy()
                .into_owned(),
            action,
        };
        ensure!(
            serde_json::to_vec(&request)?.len() as u64 <= IPC_LIMIT,
            "Template request exceeds the 2 MiB worker limit; use the terminal editor. Nothing written."
        );
        request.validate()?;
        write_atomic(&directory.path().join("request.json"), &request)?;
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let outcome = match run_owned_template(&self.worker, directory.path(), sink.clone()) {
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => {
                let _ = directory.keep();
                return Err(error);
            }
            result => result?,
        };
        let checked =
            read_json::<TemplateReport>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    report.status.is_some(),
                    "Template worker exited without a terminal report"
                );
                ensure!(
                    (outcome == OperationOutcome::Succeeded)
                        == (report.status == Some(TemplateStatus::Succeeded)),
                    "Template exit and report disagree"
                );
                Ok(report)
            });
        let report = match checked {
            Ok(report) => report,
            Err(error) => {
                let mut report = TemplateReport::initial(&request);
                let mutation = matches!(request.action, TemplateJob::Execute(_));
                report.status = Some(TemplateStatus::Uncertain);
                report.mutation = if mutation {
                    TemplateMutation::Unknown
                } else {
                    TemplateMutation::NotStarted
                };
                report.message = Some(format!(
                    "No trustworthy template completion report. {}: {error:#}",
                    if mutation {
                        "Reload from file before retrying; the override may have changed"
                    } else {
                        "No template writes were requested"
                    }
                ));
                report
            }
        };
        sink(OperationEvent::Finished {
            outcome: if report.status == Some(TemplateStatus::Succeeded) {
                OperationOutcome::Succeeded
            } else {
                OperationOutcome::Failed {
                    code: match outcome {
                        OperationOutcome::Failed { code } => code,
                        _ => None,
                    },
                }
            },
        });
        Ok(report)
    }
}

fn successful(report: TemplateReport) -> Result<TemplateReport> {
    if report.status == Some(TemplateStatus::Uncertain) {
        return Err(anyhow::Error::new(UnconfirmedCompletion).context(
            report
                .message
                .unwrap_or_else(|| "Reload the template before retrying".into()),
        ));
    }
    ensure!(
        report.status == Some(TemplateStatus::Succeeded),
        "{}",
        report
            .message
            .as_deref()
            .unwrap_or("Template preparation did not complete; retry explicitly")
    );
    Ok(report)
}

#[cfg(windows)]
fn run_owned_template(
    worker: &Path,
    directory: &Path,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    use process_wrap::tokio::{CommandWrap, CreationFlags, JobObject};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let outcome = runtime.block_on(async {
        let mut command = CommandWrap::with_new(worker, |command| {
            command
                .arg(WORKER_FLAG)
                .arg("template-job")
                .arg(directory)
                .current_dir(directory)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        });
        command.wrap(JobObject).wrap(CreationFlags(
            windows::Win32::System::Threading::CREATE_NO_WINDOW,
        ));
        let mut child = command
            .spawn()
            .context("Could not start the Project Template worker")?;
        sink(OperationEvent::Started);
        async {
            let stdout = tokio::spawn(drain(
                child.stdout().take().context("Missing worker stdout")?,
                false,
                sink.clone(),
            ));
            let stderr = tokio::spawn(drain(
                child.stderr().take().context("Missing worker stderr")?,
                true,
                sink,
            ));
            // wait covers the entire Windows job. A root exit alone cannot authorize closing.
            let status = child.wait().await?;
            tokio::time::timeout(Duration::from_secs(5), async {
                stdout.await??;
                stderr.await??;
                Ok::<_, anyhow::Error>(())
            })
            .await
            .context("Template worker output did not finish draining")??;
            Ok::<_, anyhow::Error>(if status.success() {
                OperationOutcome::Succeeded
            } else {
                OperationOutcome::Failed {
                    code: status.code(),
                }
            })
        }
        .await
        .context(OwnedTeardownUnconfirmed)
    });
    if outcome.is_err() {
        runtime.shutdown_background();
    }
    outcome
}
#[cfg(not(windows))]
fn run_owned_template(
    _: &Path,
    _: &Path,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop Project Template workers are currently supported on Windows; terminal editing remains available"
    )
}
