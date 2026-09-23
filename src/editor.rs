use std::{env, path::Path, process::Command};

use crate::error::{AppError, AppResult};

enum LineArg {
    Plus,
    Colon,
}

pub fn command(configured: Option<&str>, path: &Path, line: usize) -> AppResult<Command> {
    command_for(&resolve_editor(configured), path, line)
}

fn command_for(template: &str, path: &Path, line: usize) -> AppResult<Command> {
    let pieces = split_command(template)?;
    let has_placeholders = pieces
        .iter()
        .any(|piece| piece.contains("{file}") || piece.contains("{line}"));

    let pieces: Vec<String> = if has_placeholders {
        let file = path.display().to_string();
        let line = line.to_string();
        pieces
            .into_iter()
            .map(|piece| piece.replace("{line}", &line).replace("{file}", &file))
            .collect()
    } else {
        pieces
    };

    let Some((program, args)) = pieces.split_first() else {
        return Err(AppError::EmptyEditorCommand);
    };

    let mut cmd = Command::new(program);
    cmd.args(args);

    if !has_placeholders {
        match line_arg_style(program) {
            LineArg::Plus => {
                cmd.arg(format!("+{line}")).arg(path);
            }
            LineArg::Colon => {
                cmd.arg(format!("{}:{line}", path.display()));
            }
        }
    }
    Ok(cmd)
}

fn resolve_editor(configured: Option<&str>) -> String {
    pick_editor(configured, env::var("VISUAL").ok(), env::var("EDITOR").ok())
}

fn pick_editor(configured: Option<&str>, visual: Option<String>, editor: Option<String>) -> String {
    let is_set = |command: &String| !command.trim().is_empty();

    configured
        .map(String::from)
        .filter(is_set)
        .or(visual.filter(is_set))
        .or(editor.filter(is_set))
        .unwrap_or_else(|| "vi".into())
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

fn split_command(template: &str) -> AppResult<Vec<String>> {
    let pieces = shell_words::split(template).map_err(|_| AppError::InvalidEditorCommand {
        command: template.to_string(),
    })?;

    if pieces.is_empty() {
        Err(AppError::EmptyEditorCommand)
    } else {
        Ok(pieces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple_command() {
        let command = split_command("nvim").unwrap();
        assert_eq!(command, vec!["nvim"]);
    }

    #[test]
    fn split_complex_command() {
        let command = split_command("code --wait {file}:{line}").unwrap();
        assert_eq!(command, vec!["code", "--wait", "{file}:{line}"]);
    }

    #[test]
    fn split_command_with_space() {
        let command = split_command("'/Applications/My Editor/ed' {file}:{line}").unwrap();
        assert_eq!(command, vec!["/Applications/My Editor/ed", "{file}:{line}"]);
    }

    #[test]
    fn invalid_editor_command() {
        assert!(matches!(
            split_command("'fake_editor"),
            Err(AppError::InvalidEditorCommand { .. })
        ));
    }

    #[test]
    fn empty_editor_command() {
        assert!(matches!(
            split_command(" "),
            Err(AppError::EmptyEditorCommand)
        ));
    }

    #[test]
    fn command_for_code() {
        let cmd = command_for(
            "code --wait --goto {file}:{line}",
            Path::new("src/app.rs"),
            42,
        )
        .unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "code");
        assert_eq!(args, vec!["--wait", "--goto", "src/app.rs:42"]);
    }

    #[test]
    fn command_for_nvim() {
        let cmd = command_for("nvim +{line} {file}", Path::new("src/app.rs"), 42).unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "nvim");
        assert_eq!(args, vec!["+42", "src/app.rs"]);
    }

    #[test]
    fn command_for_nvim_without_placeholders() {
        let cmd = command_for("nvim", Path::new("src/app.rs"), 42).unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "nvim");
        assert_eq!(args, vec!["+42", "src/app.rs"]);
    }

    #[test]
    fn command_for_hx() {
        let cmd = command_for("hx", Path::new("src/app.rs"), 42).unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "hx");
        assert_eq!(args, vec!["src/app.rs:42"]);
    }

    #[test]
    fn command_for_code_with_wait() {
        let cmd = command_for("code --wait", Path::new("src/app.rs"), 42).unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "code");
        assert_eq!(args, vec!["--wait", "+42", "src/app.rs"]);
    }

    #[test]
    fn command_for_nvim_with_space() {
        let cmd = command_for("nvim {file}", Path::new("docs/my notes.md"), 1).unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "nvim");
        assert_eq!(args, vec!["docs/my notes.md"]);
    }

    #[test]
    fn command_for_nvim_with_file() {
        let cmd = command_for("nvim {file}", Path::new("{line}.md"), 42).unwrap();

        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(cmd.get_program(), "nvim");
        assert_eq!(args, vec!["{line}.md"]);
    }

    #[test]
    fn configured_editor_wins_over_environment() {
        let editor = pick_editor(Some("hx"), Some("nvim".into()), Some("vim".into()));
        assert_eq!(editor, "hx");
    }

    #[test]
    fn visual_wins_over_editor() {
        let editor = pick_editor(None, Some("nvim".into()), Some("vim".into()));
        assert_eq!(editor, "nvim");
    }

    #[test]
    fn editor_is_used_when_visual_is_unset() {
        let editor = pick_editor(None, None, Some("vim".into()));
        assert_eq!(editor, "vim");
    }

    #[test]
    fn falls_back_to_vi() {
        let editor = pick_editor(None, None, None);
        assert_eq!(editor, "vi");
    }

    #[test]
    fn blank_values_are_skipped() {
        let editor = pick_editor(Some(""), Some("  ".into()), Some("vim".into()));
        assert_eq!(editor, "vim");
    }
}
