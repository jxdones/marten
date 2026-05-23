use std::collections::BTreeMap;

use crate::git::repository::FileEntry;

pub enum TreeRow<'a> {
    File(&'a FileEntry, usize),
    Dir(String, usize),
}

pub enum FileNode<'a> {
    File(&'a FileEntry),
    Dir(String, BTreeMap<String, Self>),
}

const INITIAL_DEPTH: usize = 0;
const ONE_CHILD_ONLY: usize = 1;

pub fn tree_rows(files: &[FileEntry]) -> Vec<TreeRow<'_>> {
    let mut root = FileNode::Dir(String::from("/"), BTreeMap::new());

    for entry in files {
        let segments = entry.path.split('/').collect::<Vec<&str>>();
        insert(&mut root, entry, &segments);
    }
    render(&root, INITIAL_DEPTH)
}

fn insert<'a>(node: &mut FileNode<'a>, entry: &'a FileEntry, segments: &[&str]) {
    match node {
        FileNode::Dir(_dir_name, children) => match segments {
            [] => {}
            [name] => {
                let child = FileNode::File(entry);
                children.insert(name.to_string(), child);
            }
            [first, rest @ ..] => {
                let child = children
                    .entry(first.to_string())
                    .or_insert_with(|| FileNode::Dir(first.to_string(), BTreeMap::new()));
                insert(child, entry, rest);
            }
        },
        FileNode::File(_) => {}
    }
}

fn render<'a>(node: &FileNode<'a>, depth: usize) -> Vec<TreeRow<'a>> {
    match node {
        FileNode::File(entry) => {
            vec![TreeRow::File(entry, depth)]
        }
        FileNode::Dir(name, children) => {
            if name == "/" {
                let mut branch_rows = vec![];
                for child in children.values() {
                    branch_rows.extend(render(child, depth + 1));
                }
                return branch_rows;
            }
            if children.len() == ONE_CHILD_ONLY {
                let next_child = children.values().next().unwrap();

                match next_child {
                    FileNode::Dir(child_name, child_children) => {
                        let collapsed_dir = format!("{name}/{child_name}");
                        let mut branch_rows = vec![TreeRow::Dir(collapsed_dir, depth)];

                        for child in child_children.values() {
                            branch_rows.extend(render(child, depth + 1));
                        }
                        branch_rows
                    }
                    FileNode::File(entry) => {
                        vec![TreeRow::File(entry, depth)]
                    }
                }
            } else {
                let mut branch_rows = vec![TreeRow::Dir(name.clone(), depth)];
                for child in children.values() {
                    branch_rows.extend(render(child, depth + 1));
                }
                branch_rows
            }
        }
    }
}
