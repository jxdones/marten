use std::collections::{BTreeMap, HashSet};

use crate::state::FileSlot;

pub enum TreeRow {
    File(usize, usize),
    Dir(String, usize),
}

pub enum FileNode {
    File(usize),
    Dir(String, BTreeMap<String, Self>),
}

const INITIAL_DEPTH: usize = 0;
const ONE_CHILD_ONLY: usize = 1;

pub fn tree_rows(files: &[FileSlot], collapsed: &HashSet<String>) -> Vec<TreeRow> {
    let mut root = FileNode::Dir(String::from("/"), BTreeMap::new());

    for (idx, slot) in files.iter().enumerate() {
        let segments = slot.entry.path.split('/').collect::<Vec<&str>>();
        insert(&mut root, &segments, idx);
    }
    render(&root, INITIAL_DEPTH, "", collapsed)
}

fn insert(node: &mut FileNode, segments: &[&str], index: usize) {
    match node {
        FileNode::Dir(_dir_name, children) => match segments {
            [] => {}
            [name] => {
                children.insert(name.to_string(), FileNode::File(index));
            }
            [first, rest @ ..] => {
                let child = children
                    .entry(first.to_string())
                    .or_insert_with(|| FileNode::Dir(first.to_string(), BTreeMap::new()));
                insert(child, rest, index);
            }
        },
        FileNode::File(_) => {}
    }
}

fn render(node: &FileNode, depth: usize, path: &str, collapsed: &HashSet<String>) -> Vec<TreeRow> {
    match node {
        FileNode::File(idx) => {
            vec![TreeRow::File(*idx, depth)]
        }
        FileNode::Dir(name, children) => {
            if name == "/" {
                let mut branch_rows = vec![];
                for child in children.values().filter(|c| matches!(c, FileNode::Dir(..))) {
                    branch_rows.extend(render(child, depth + 1, "", collapsed));
                }
                for child in children.values().filter(|c| matches!(c, FileNode::File(_))) {
                    branch_rows.extend(render(child, depth + 1, "", collapsed));
                }
                return branch_rows;
            }
            if children.len() == ONE_CHILD_ONLY {
                let next_child = children.values().next().unwrap();

                match next_child {
                    FileNode::Dir(child_name, child_children) => {
                        let fullpath = if path.is_empty() {
                            name.clone()
                        } else {
                            format!("{path}/{name}")
                        };
                        let collapsed_dir = format!("{fullpath}/{child_name}");

                        if collapsed.contains(&collapsed_dir) {
                            return vec![TreeRow::Dir(collapsed_dir, depth)];
                        }

                        let mut branch_rows = vec![TreeRow::Dir(collapsed_dir.clone(), depth)];

                        for child in child_children.values() {
                            branch_rows.extend(render(child, depth + 1, &collapsed_dir, collapsed));
                        }
                        branch_rows
                    }
                    FileNode::File(idx) => {
                        let fullpath = if path.is_empty() {
                            name.clone()
                        } else {
                            format!("{path}/{name}")
                        };
                        if collapsed.contains(&fullpath) {
                            return vec![TreeRow::Dir(fullpath, depth)];
                        }
                        vec![
                            TreeRow::Dir(fullpath, depth),
                            TreeRow::File(*idx, depth + 1),
                        ]
                    }
                }
            } else {
                let fullpath = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}/{name}")
                };

                if collapsed.contains(&fullpath) {
                    return vec![TreeRow::Dir(fullpath, depth)];
                }

                let mut branch_rows = vec![TreeRow::Dir(fullpath.clone(), depth)];
                for child in children.values().filter(|c| matches!(c, FileNode::Dir(..))) {
                    branch_rows.extend(render(child, depth + 1, &fullpath, collapsed));
                }
                for child in children.values().filter(|c| matches!(c, FileNode::File(_))) {
                    branch_rows.extend(render(child, depth + 1, &fullpath, collapsed));
                }
                branch_rows
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repository::{FileEntry, FileStatus};
    use crate::state::DiffLoadState;

    fn file_entry(path: &str) -> FileSlot {
        FileSlot {
            entry: FileEntry {
                path: path.to_string(),
                status: FileStatus::Untracked,
                insertions: 0,
                deletions: 0,
            },
            load: DiffLoadState::NotLoaded,
        }
    }

    fn dir_keys(rows: &[TreeRow]) -> Vec<&str> {
        rows.iter()
            .filter_map(|row| match row {
                TreeRow::Dir(key, _) => Some(key.as_str()),
                TreeRow::File(_, _) => None,
            })
            .collect()
    }

    #[test]
    fn nested_dir_uses_full_path_as_key() {
        let files = vec![
            file_entry(".marten-dev/src/app.rs"),
            file_entry(".marten-dev/src/helper/common/common.rs"),
        ];

        let rows = tree_rows(&files, &HashSet::new());
        let keys = dir_keys(&rows);
        assert!(keys.contains(&".marten-dev/src"));
        assert!(keys.contains(&".marten-dev/src/helper/common"));
    }

    #[test]
    fn single_child_dir_compression_uses_full_path() {
        let files = vec![
            file_entry(".marten-dev/tests/fixtures/fixtures.rs"),
            file_entry(".marten-dev/tests/fixtures/helpers.rs"),
        ];

        let rows = tree_rows(&files, &HashSet::new());
        let keys = dir_keys(&rows);
        assert!(keys.contains(&".marten-dev/tests/fixtures"));
    }

    #[test]
    fn collapsing_a_nested_dir_hides_its_children() {
        let files = vec![
            file_entry(".marten-dev/src/app.rs"),
            file_entry(".marten-dev/src/lib.rs"),
        ];

        let rows = tree_rows(&files, &HashSet::from([".marten-dev/src".to_string()]));
        let file_count = rows
            .iter()
            .filter(|row| matches!(row, TreeRow::File(..)))
            .count();
        assert_eq!(file_count, 0);
    }

    #[test]
    fn top_level_dir_collapses_correctly() {
        let files = vec![
            file_entry(".marten-dev/src/app.rs"),
            file_entry(".marten-dev/src/state/file.rs"),
            file_entry(".marten-dev/tests/fixtures.rs"),
        ];
        let rows = tree_rows(&files, &HashSet::from([".marten-dev".to_string()]));
        let file_count = rows
            .iter()
            .filter(|row| matches!(row, TreeRow::File(..)))
            .count();
        assert_eq!(file_count, 0);
    }
}
