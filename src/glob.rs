pub fn pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }

    let pattern = if pattern.ends_with('/') {
        format!("{}**", pattern)
    } else {
        pattern.to_string()
    };

    let segments: Vec<&str> = pattern.split('/').collect();

    if segments.len() == 1 {
        let basename = path.rsplit('/').next().unwrap_or(path);
        return segment_matches(segments[0], basename);
    }

    let path_segments: Vec<&str> = path.split('/').collect();
    segments_match(&segments, &path_segments)
}

pub fn matches_any(patterns: &[String], path: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| pattern_matches(pattern, path))
}

fn segments_match(segments: &[&str], path_segments: &[&str]) -> bool {
    match segments.split_first() {
        None => path_segments.is_empty(),
        Some((segment, rest)) if *segment == "**" => {
            segments_match(rest, path_segments)
                || path_segments
                    .split_first()
                    .is_some_and(|(_, tail)| segments_match(segments, tail))
        }
        Some((segment, rest)) => match path_segments.split_first() {
            Some((part, tail)) => segment_matches(segment, part) && segments_match(rest, tail),
            None => false,
        },
    }
}

fn segment_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    chars_match(&pattern, &text)
}

fn chars_match(pattern: &[char], text: &[char]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => {
            chars_match(rest, text)
                || text
                    .split_first()
                    .is_some_and(|(_, tail)| chars_match(pattern, tail))
        }
        Some(('?', rest)) => match text.split_first() {
            Some((_, tail)) => chars_match(rest, tail),
            None => false,
        },
        Some(('[', rest)) => match parse_class(rest) {
            Some((members, negated, consumed)) => match text.split_first() {
                Some((text_char, tail)) => {
                    let ok = if negated {
                        !members.contains(text_char)
                    } else {
                        members.contains(text_char)
                    };
                    ok && chars_match(&pattern[1 + consumed..], tail)
                }
                None => false,
            },
            None => match text.split_first() {
                Some((text_char, tail)) => *text_char == '[' && chars_match(rest, tail),
                None => false,
            },
        },
        Some((expected, rest)) => match text.split_first() {
            Some((actual, tail)) => expected == actual && chars_match(rest, tail),
            None => false,
        },
    }
}

fn parse_class(pattern: &[char]) -> Option<(Vec<char>, bool, usize)> {
    let mut i = 0;
    let mut negated = false;

    if pattern.first().is_some_and(|c| *c == '!' || *c == '^') {
        negated = true;
        i += 1;
    }

    let mut members = Vec::new();
    loop {
        let start = *pattern.get(i)?;
        i += 1;
        if start == ']' {
            break;
        }
        if pattern.get(i) == Some(&'-') {
            if pattern.get(i + 1) == Some(&']') {
                members.push(start);
                members.push('-');
                i += 1;
            } else {
                let end = *pattern.get(i + 1)?;
                if start <= end {
                    members.extend(start..=end);
                }
                i += 2;
            }
        } else {
            members.push(start);
        }
    }

    Some((members, negated, i))
}

#[cfg(test)]
mod tests {
    use super::pattern_matches as matches;

    #[test]
    fn literal_path_matches_exactly() {
        assert!(matches("src/main.rs", "src/main.rs"));
        assert!(!matches("src/main.rs", "src/lib.rs"));
        assert!(!matches("src/main.rs", "backend/src/main.rs"));
    }

    #[test]
    fn no_slash_pattern_matches_basename_at_any_depth() {
        assert!(matches("*.lock", "Cargo.lock"));
        assert!(matches("*.lock", "backend/Cargo.lock"));
        assert!(matches("*.lock", "a/b/c/Cargo.lock"));
        assert!(!matches("*.lock", "Cargo.toml"));
    }

    #[test]
    fn star_does_not_cross_segments() {
        assert!(matches("src/*.rs", "src/main.rs"));
        assert!(!matches("src/*.rs", "src/sub/main.rs"));
    }

    #[test]
    fn double_star_matches_any_depth() {
        assert!(matches("**/generated/**", "generated/out.js"));
        assert!(matches("**/generated/**", "src/generated/out.js"));
        assert!(matches("**/generated/**", "src/a/generated/b/out.js"));
        assert!(!matches("**/generated/**", "src/out.js"));
        assert!(matches("**/README.md", "README.md"));
        assert!(matches("**/README.md", "docs/README.md"));
    }

    #[test]
    fn double_star_can_match_zero_segments() {
        assert!(matches("src/**", "src/main.rs"));
        assert!(matches("src/**", "src/deep/main.rs"));
        assert!(matches("src/**/main.rs", "src/main.rs"));
        assert!(matches("src/**/main.rs", "src/a/b/main.rs"));
    }

    #[test]
    fn trailing_slash_matches_everything_beneath() {
        assert!(matches("vendor/", "vendor/x.js"));
        assert!(matches("vendor/", "vendor/deep/y.js"));
        assert!(!matches("vendor/", "src/vendor/x.js"));
    }

    #[test]
    fn question_mark_matches_single_char() {
        assert!(matches("Cargo.l?ck", "Cargo.lock"));
        assert!(!matches("Cargo.l?ck", "Cargo.lcks"));
        assert!(!matches("a?c", "ac"));
    }

    #[test]
    fn char_class_matches_ranges() {
        assert!(matches("file[0-9].txt", "file3.txt"));
        assert!(!matches("file[0-9].txt", "filex.txt"));
        assert!(matches("file[a-c].txt", "fileb.txt"));
    }

    #[test]
    fn negated_char_class() {
        assert!(matches("file[!0-9].txt", "filex.txt"));
        assert!(!matches("file[!0-9].txt", "file3.txt"));
        assert!(matches("file[^0-9].txt", "filex.txt"));
    }

    #[test]
    fn unclosed_class_is_literal() {
        assert!(matches("weird[file", "weird[file"));
        assert!(!matches("weird[file", "weirdfile"));
    }

    #[test]
    fn patterns_are_case_sensitive() {
        assert!(!matches("*.LOCK", "Cargo.lock"));
    }

    #[test]
    fn empty_and_whitespace_patterns_never_match() {
        assert!(!matches("", "Cargo.lock"));
        assert!(!matches("   ", "Cargo.lock"));
    }

    #[test]
    fn wildcard_in_directory_segment() {
        assert!(matches("gen*/", "generated/a.txt"));
        assert!(matches("gen*/", "genx/deep/a.txt"));
        assert!(!matches("gen*/", "src/generated/a.txt"));
    }
}
