use similar::{ChangeTag, TextDiff};

pub fn changed_ranges(old: &str, new: &str) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
    let diff = TextDiff::from_unicode_words(old, new);
    let mut old_ranges = Vec::new();
    let mut new_ranges = Vec::new();
    let mut old_pos = 0;
    let mut new_pos = 0;

    for change in diff.iter_all_changes() {
        let len = change.value().chars().count();
        match change.tag() {
            ChangeTag::Delete => {
                old_ranges.push((old_pos, old_pos + len));
                old_pos += len;
            }
            ChangeTag::Insert => {
                new_ranges.push((new_pos, new_pos + len));
                new_pos += len;
            }
            ChangeTag::Equal => {
                old_pos += len;
                new_pos += len;
            }
        }
    }

    (merge_ranges(old_ranges), merge_ranges(new_ranges))
}

fn merge_ranges(ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();

    for (start, end) in ranges.into_iter().filter(|(start, end)| start < end) {
        if let Some((_, prev_end)) = merged.last_mut()
            && start <= *prev_end
        {
            *prev_end = (*prev_end).max(end);
            continue;
        }

        merged.push((start, end));
    }

    merged
}
