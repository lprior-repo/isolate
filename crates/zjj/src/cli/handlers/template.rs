//! Template command handlers

use anyhow::Result;
use clap::ArgMatches;
use zjj_core::{zellij::LayoutTemplate, OutputFormat};

use crate::commands::template;

pub async fn handle_template(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", sub)) => {
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            template::run_list(format).await
        }
        Some(("create", sub)) => {
            let name = sub
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Template name is required"))?
                .clone();
            let description = sub.get_one::<String>("description").cloned();
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            let source = if let Some(file_path) = sub.get_one::<String>("from-file") {
                template::TemplateSource::FromFile(file_path.clone())
            } else if let Some(builtin) = sub.get_one::<String>("builtin") {
                let template_type = match builtin.as_str() {
                    "minimal" => LayoutTemplate::Minimal,
                    "standard" => LayoutTemplate::Standard,
                    "full" => LayoutTemplate::Full,
                    "split" => LayoutTemplate::Split,
                    "review" => LayoutTemplate::Review,
                    _ => return Err(anyhow::anyhow!("Invalid builtin template: {builtin}")),
                };
                template::TemplateSource::Builtin(template_type)
            } else {
                template::TemplateSource::Builtin(LayoutTemplate::Standard)
            };
            template::run_create(&template::CreateOptions {
                name,
                description,
                source,
                format,
            })
            .await
        }
        Some(("show", sub)) => {
            let name = sub
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Template name is required"))?;
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            template::run_show(name, format).await
        }
        Some(("delete", sub)) => {
            let name = sub
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Template name is required"))?;
            let force = sub.get_flag("force");
            let json = sub.get_flag("json");
            let format = OutputFormat::from_json_flag(json);
            template::run_delete(name, force, format).await
        }
        _ => Err(anyhow::anyhow!("Invalid template subcommand")),
    }
}
