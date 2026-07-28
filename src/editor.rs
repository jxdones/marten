use std::{env, path::Path, process::Command};

enum LineArg {
    Plus,
    Colon,
}

pub fn command(path: &Path, line: usize) -> Command {
    command_for(&resolve_editor(), path, line)
}

fn command_for(editor: &str, path: &Path, line: usize) -> Command {
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

fn resolve_editor() -> String {
    env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into())
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
