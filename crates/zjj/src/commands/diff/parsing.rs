//! Parse diff output into structured format

use crate::json_output::{DiffStat, FileDiffStat};

/// Parse jj diff --stat output into structured `DiffStat`
pub fn parse_diff_stat(output: &str) -> DiffStat {
    let files: Vec<FileDiffStat> = output
        .lines()
        .filter_map(|line| {
            line.split_once('|').map(|(path_part, rest)| {
                let path = path_part.trim().to_string();
                let insertions = rest.chars().filter(|&c| c == '+').count();
                let deletions = rest.chars().filter(|&c| c == '-').count();

                let status = match (insertions > 0, deletions > 0) {
                    (true, true) => "modified".to_string(),
                    (true, false) => "added".to_string(),
                    (false, true) => "deleted".to_string(),
                    (false, false) => "unchanged".to_string(),
                };

                FileDiffStat {
                    path,
                    insertions,
                    deletions,
                    status,
                }
            })
        })
        .collect();

    let (total_insertions, total_deletions) =
        files.iter().fold((0usize, 0usize), |(ins, del), file| {
            (
                ins.saturating_add(file.insertions),
                del.saturating_add(file.deletions),
            )
        });

    DiffStat {
        files_changed: files.len(),
        insertions: total_insertions,
        deletions: total_deletions,
        files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_stat_empty() {
        let stat = parse_diff_stat("");
        assert_eq!(stat.files_changed, 0);
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 0);
        assert!(stat.files.is_empty());
    }

    #[test]
    fn test_parse_diff_stat_additions() {
        let output = "src/main.rs | 10 ++++++++++\n";
        let stat = parse_diff_stat(output);
        assert_eq!(stat.files_changed, 1);
        assert_eq!(stat.insertions, 10);
        assert_eq!(stat.deletions, 0);
        assert_eq!(stat.files[0].path, "src/main.rs");
        assert_eq!(stat.files[0].status, "added");
    }

    #[test]
    fn test_parse_diff_stat_deletions() {
        let output = "src/old.rs | 5 -----\n";
        let stat = parse_diff_stat(output);
        assert_eq!(stat.files_changed, 1);
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 5);
        assert_eq!(stat.files[0].status, "deleted");
    }

    #[test]
    fn test_parse_diff_stat_modifications() {
        let output = "src/lib.rs | 8 ++++----\n";
        let stat = parse_diff_stat(output);
        assert_eq!(stat.files_changed, 1);
        assert_eq!(stat.insertions, 4);
        assert_eq!(stat.deletions, 4);
        assert_eq!(stat.files[0].status, "modified");
    }

    #[test]
    fn test_parse_diff_stat_multiple_files() {
        let output = "src/main.rs | 5 +++++\nsrc/lib.rs | 3 +--\nREADME.md | 2 --\n";
        let stat = parse_diff_stat(output);
        assert_eq!(stat.files_changed, 3);
        assert_eq!(stat.insertions, 6); // 5 + 1
        assert_eq!(stat.deletions, 4); // 2 + 2
    }
}
