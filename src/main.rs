#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod startup;

use clap::{Parser, Subcommand};
use oj_agent::{
    app,
    error::AppError,
    ir::*,
    session::{SessionStore, SqliteStore},
    tools,
};
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[derive(Parser)]
#[command(
    version,
    about = "OJ algorithm animation workbench. Run without arguments for the desktop UI."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    /// Generate a demo, or read a serialized ProblemInput.
    Generate {
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long, default_value = "output")]
        output: PathBuf,
        #[arg(long)]
        gif: bool,
    },
    /// Validate an animation document.
    Validate { path: PathBuf },
    /// Render an animation as GIF or PNG.
    Render {
        path: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = 0.)]
        time: f64,
        /// Appearance JSON shared with desktop previews.
        #[arg(long)]
        style: Option<PathBuf>,
    },
    /// List namespaced tools and JSON schemas, or invoke a tool with JSON input.
    Tool {
        name: Option<String>,
        #[arg(long)]
        input: Option<PathBuf>,
    },
    /// Export the versioned JSON schemas.
    Schemas {
        #[arg(default_value = "schemas")]
        output: PathBuf,
    },
}
fn read_json(path: &std::path::Path) -> anyhow::Result<serde_json::Value> {
    let metadata = std::fs::metadata(path)?;
    anyhow::ensure!(
        metadata.len() <= 16 * 1024 * 1024,
        "JSON file exceeds 16 MB"
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    if std::env::args_os().nth(1).is_some() {
        startup::attach_parent_console();
    }
    match Cli::parse().command {
        None => {
            let result = app::run();
            #[cfg(windows)]
            if let Err(error) = &result {
                // Shortcut launches have no stderr console in which to show startup failures.
                startup::show_error(error);
            }
            result
        }
        Some(Command::Generate { input, output, gif }) => {
            let input = if let Some(path) = input {
                parse_validated::<ProblemInput>(&read_json(&path)?)?
            } else {
                ProblemInput::default()
            };
            std::fs::create_dir_all(&output)?;
            let mut store = SqliteStore::open(&output.join("sessions.sqlite3"))?;
            let id = store.create(&input)?;
            let runtime = tokio::runtime::Runtime::new()?;
            let g = runtime.block_on(tools::generate(
                &mut store,
                &id,
                &input,
                None,
                &AtomicBool::new(false),
                |p| eprintln!("{p}"),
            ))?;
            for artifact in store.artifacts(&id)? {
                std::fs::write(
                    output.join(format!("{}-v{}.json", artifact.kind, artifact.revision)),
                    serde_json::to_vec_pretty(&artifact.document)?,
                )?;
            }
            oj_agent::export::png(
                &g.animation,
                &output.join("preview.png"),
                g.animation.step_times[1],
            )?;
            store.record_export(&id, &output.join("preview.png"), "png")?;
            if gif {
                oj_agent::export::gif(&g.animation, &output.join("preview.gif"), 30)?;
                store.record_export(&id, &output.join("preview.gif"), "gif")?;
            }
            println!("{}", serde_json::to_string_pretty(&g)?);
            Ok(())
        }
        Some(Command::Validate { path }) => {
            let animation: AnimationIr = parse_validated(&read_json(&path)?)?;
            animation.validate()?;
            println!("Valid animation-ir v1");
            Ok(())
        }
        Some(Command::Render {
            path,
            output,
            time,
            style,
        }) => {
            let animation: AnimationIr = parse_validated(&read_json(&path)?)?;
            let style = if let Some(path) = style {
                anyhow::ensure!(
                    path.is_file(),
                    "Style config does not exist: {}",
                    path.display()
                );
                oj_agent::style::StyleConfig::load(&path)?
            } else {
                oj_agent::style::StyleConfig::default()
            };
            if output.extension().is_some_and(|e| e == "gif") {
                oj_agent::export::gif_cancellable_styled(
                    &animation,
                    &output,
                    30,
                    &AtomicBool::new(false),
                    &style,
                )?;
            } else {
                oj_agent::export::png_styled(&animation, &output, time, &style)?;
            }
            Ok(())
        }
        Some(Command::Tool { name, input }) => {
            if let Some(name) = name {
                let input = input.ok_or_else(|| {
                    AppError::new(
                        "INPUT_MISSING",
                        "Input",
                        "ToolDispatch",
                        "Pass --input path.json",
                    )
                })?;
                let result = tools::invoke(&name, &read_json(&input)?);
                println!("{}", serde_json::to_string_pretty(&result)?);
                anyhow::ensure!(result["ok"] == true, "Tool invocation failed");
            } else {
                println!("{}", serde_json::to_string_pretty(&tools::definitions())?);
            }
            Ok(())
        }
        Some(Command::Schemas { output }) => {
            std::fs::create_dir_all(&output)?;
            for (name, value) in [
                ("problem", schema::<ProblemInput>()),
                ("algorithm", schema::<AlgorithmIr>()),
                ("trace", schema::<TraceIr>()),
                ("visualization", schema::<VisualizationIr>()),
                ("animation", schema::<AnimationIr>()),
                ("observation", schema::<oj_sandbox::Step>()),
            ] {
                std::fs::write(
                    output.join(format!("{name}-v1.schema.json")),
                    serde_json::to_vec_pretty(&value)?,
                )?;
            }
            Ok(())
        }
    }
}
