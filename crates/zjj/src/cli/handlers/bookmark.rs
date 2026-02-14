//! Bookmark command handlers

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::bookmark;

pub async fn handle_bookmark(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", list_m)) => {
            let session = list_m.get_one::<String>("session").cloned();
            let show_all = list_m.get_flag("all");
            let format = get_format(list_m);
            bookmark::run_list(&bookmark::ListOptions {
                session,
                show_all,
                format,
            })
            .await
        }
        Some(("create", create_m)) => {
            let name = create_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Bookmark name is required"))?
                .clone();
            let session = create_m.get_one::<String>("session").cloned();
            let push = create_m.get_flag("push");
            let format = get_format(create_m);
            bookmark::run_create(&bookmark::CreateOptions {
                name,
                session,
                push,
                format,
            })
            .await
        }
        Some(("delete", delete_m)) => {
            let name = delete_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Bookmark name is required"))?
                .clone();
            let session = delete_m.get_one::<String>("session").cloned();
            let format = get_format(delete_m);
            bookmark::run_delete(&bookmark::DeleteOptions {
                name,
                session,
                format,
            })
            .await
        }
        Some(("move", move_m)) => {
            let name = move_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Bookmark name is required"))?
                .clone();
            let to_revision = move_m
                .get_one::<String>("to")
                .ok_or_else(|| anyhow::anyhow!("Target revision (--to) is required"))?
                .clone();
            let session = move_m.get_one::<String>("session").cloned();
            let format = get_format(move_m);
            bookmark::run_move(&bookmark::MoveOptions {
                name,
                to_revision,
                session,
                format,
            })
            .await
        }
        _ => Err(anyhow::anyhow!(
            "Subcommand required: list, create, delete, or move"
        )),
    }
}
