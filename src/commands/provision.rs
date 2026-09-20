use crate::config::GlobalConfig;
use anyhow::Result;

pub fn run(config: &mut GlobalConfig) -> Result<()> {
    super::machine_setup::run(config)
}
