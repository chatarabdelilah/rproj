pub(crate) mod checked;
#[cfg(test)]
mod fixtures;
mod model;
pub(crate) mod worker;

use anyhow::{Result, bail, ensure};
use inquire::{Confirm, MultiSelect};
use rproj_core::machine_setup::{Category, Outcome, Persistence};
use std::{io::IsTerminal, sync::Arc};

use crate::{config::GlobalConfig, interrupt, steps::execution::MessageKind, ui};

pub fn open() -> Result<()> {
    run(&mut GlobalConfig::default())
}

pub fn run(config: &mut GlobalConfig) -> Result<()> {
    ensure!(
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        "Machine Setup requires an interactive terminal; run `rproj setup` in a terminal"
    );
    let context = checked::capture()?;
    let mut selection = context.saved.clone();
    for category in Category::ALL {
        if !selection.active(category) {
            continue;
        }
        let choices: Vec<_> = category
            .entries()
            .into_iter()
            .map(|entry| entry.key)
            .collect();
        let defaults: Vec<_> = choices
            .iter()
            .enumerate()
            .filter_map(|(index, key)| selection.keys(category).contains(*key).then_some(index))
            .collect();
        let chosen = MultiSelect::new(category.label(), choices)
            .with_default(&defaults)
            .prompt_skippable()?
            .ok_or(interrupt::Cancelled)?;
        selection.replace_category(category, &chosen.into_iter().map(str::to_owned).collect());
    }

    let review = checked::prepare(&context, &selection)?;
    ui::section("Machine Setup review");
    println!("Projects folder: {}", review.selection.root.display());
    for item in &review.items {
        println!("  {}", item.label());
    }
    if !Confirm::new("Apply Machine Setup?")
        .with_default(false)
        .prompt_skippable()?
        .unwrap_or(false)
    {
        return Err(interrupt::Cancelled.into());
    }

    interrupt::install()?;
    let items = review.items.clone();
    let result = checked::execute(
        &review,
        &interrupt::requested,
        Arc::new(move |event| match event {
            worker::Event::Started(index) => ui::section(items[index].label()),
            worker::Event::Finished(index, status) => {
                println!("  {}: {status:?}", items[index].label())
            }
            worker::Event::Message(MessageKind::Warning, text) => ui::warn(&text),
            worker::Event::Message(_, text) => println!("{text}"),
            #[cfg(test)]
            worker::Event::Done(_) | worker::Event::Completed(_) => {}
        }),
        &mut || Ok(()),
    )?;
    match result.outcome {
        Outcome::Completed | Outcome::Warnings => {}
        Outcome::Failed => bail!(
            "Machine Setup failed; see {}",
            crate::diagnostics::path_message()
        ),
        Outcome::Cancelled => return Err(interrupt::Cancelled.into()),
    }
    match result.persistence {
        Persistence::Saved { contents } => *config = toml::from_str(&contents)?,
        Persistence::Failed { message } => bail!("{message}"),
        Persistence::NotSaved => bail!("Setup did not confirm configuration persistence"),
    }
    crate::steps::notify::summary(
        "rproj setup",
        "Setup finished. Review any warnings and manual steps.",
    );
    Ok(())
}
