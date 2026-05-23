use std::collections::{BTreeMap, HashSet};

use crate::git::repository::FileEntry;

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

pub fn tree_rows(files: &[FileEntry], collapsed: &HashSet<String>) -> Vec<TreeRow> {
    let mut root = FileNode::Dir(String::from("/"), BTreeMap::new());

    for (idx, entry) in files.iter().enumerate() {
        let segments = entry.path.split('/').collect::<Vec<&str>>();
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
                for child in children.values() {
                    branch_rows.extend(render(child, depth + 1, "", collapsed));
                }
                return branch_rows;
            }
            if children.len() == ONE_CHILD_ONLY {
                let next_child = children.values().next().unwrap();

                match next_child {
                    FileNode::Dir(child_name, child_children) => {
                        let collapsed_dir = format!("{name}/{child_name}");

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
                        vec![TreeRow::File(*idx, depth)]
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

                let mut branch_rows = vec![TreeRow::Dir(name.clone(), depth)];
                for child in children.values() {
                    branch_rows.extend(render(child, depth + 1, &fullpath, collapsed));
                }
                branch_rows
            }
        }
    }
}
