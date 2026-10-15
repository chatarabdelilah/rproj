#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod workbench;

use gpui_kit::*;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.as_slice() == ["--version"] {
        println!("rproj-desktop {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if arguments.as_slice() == ["--self-check"] {
        let worker = workbench::worker_path()?;
        rproj_core::operations::WorkbenchClient::new(worker)?;
        println!(
            "{{\"desktop_version\":\"{}\",\"matching_cli\":true}}",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    let preferences_path = if arguments
        .first()
        .is_some_and(|argument| argument == "--preferences-path")
    {
        anyhow::ensure!(
            arguments.len() >= 2,
            "--preferences-path requires a file path"
        );
        let path = PathBuf::from(arguments.remove(1));
        arguments.remove(0);
        Some(path)
    } else {
        None
    };
    anyhow::ensure!(
        arguments.len() <= 1,
        "Usage: rproj-desktop [--preferences-path file] [project-folder]"
    );
    let initial_project = arguments.first().map(PathBuf::from);
    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1120.), px(780.)),
                    cx,
                ))),
                window_min_size: Some(size(px(800.), px(580.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("rproj".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            if let Err(error) = gpui_kit::open_window(options, cx, move |window, cx| {
                cx.new(|cx| {
                    workbench::Workbench::new(initial_project, preferences_path, window, cx)
                })
            }) {
                eprintln!("Could not open rproj: {error:#}");
                cx.quit();
            }
        });
    Ok(())
}
