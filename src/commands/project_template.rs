use anyhow::{Result, ensure};
use std::io::IsTerminal;

pub fn run() -> Result<()> {
    ensure!(
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        "Project Template requires an interactive terminal; open Project Template in the desktop workbench or run `rproj configure project` in a terminal"
    );
    super::app::open_project_template()
}
