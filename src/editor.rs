use std::{env, path::Path, process::Command};

enum LineArg {
    Plus,
    Colon,
}

#[derive(Debug)]
pub enum Source {
    Variable(&'static str),
    Fallback,
}

pub fn resolve() -> (String, Source) {
    for variable in ["VISUAL", "EDITOR"] {
        if let Ok(editor) = env::var(variable) {
            if !editor.trim().is_empty() {
                return (editor, Source::Variable(variable));
            }
        }
    }

    ("vi".into(), Source::Fallback)
}

pub fn command(editor: &str, path: &Path, line: usize) -> Command {
    let mut cmd = Command::new(editor);

    match line_arg_style(editor) {
        LineArg::Plus => {
            cmd.arg(format!("+{line}")).arg(path);
        }
        LineArg::Colon => {
            cmd.arg(format!("{}:{line}", path.display()));
        }
    }
    cmd
}

fn line_arg_style(editor: &str) -> LineArg {
    let basename = Path::new(editor)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(editor);

    match basename {
        "hx" => LineArg::Colon,
        _ => LineArg::Plus,
    }
}
