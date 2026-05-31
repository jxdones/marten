use similar::{ChangeTag, TextDiff};

pub type Range = (usize, usize);
pub type Ranges = Vec<Range>;

pub fn changed_ranges(old: &str, new: &str) -> (Ranges, Ranges) {
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

fn merge_ranges(ranges: Ranges) -> Ranges {
    let mut merged: Ranges = Vec::new();

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

#[cfg(test)]
mod tests {
    use super::{changed_ranges, merge_ranges};

    #[test]
    fn identical_strings_have_no_changed_ranges() {
        let (old, new) = changed_ranges("hello world", "hello world");
        assert!(old.is_empty());
        assert!(new.is_empty());
    }

    #[test]
    fn single_word_replacement() {
        let (old, new) = changed_ranges("fn old_name()", "fn new_name()");
        assert_eq!(old, vec![(3, 11)]);
        assert_eq!(new, vec![(3, 11)]);
    }

    #[test]
    fn insertion_only() {
        let (old, new) = changed_ranges("alpha beta", "alpha new beta");
        assert!(old.is_empty());
        assert_eq!(new.len(), 1);
        assert_eq!("alpha new beta"[new[0].0..new[0].1].trim(), "new");
    }

    #[test]
    fn deletion_only() {
        let (old, new) = changed_ranges("alpha new beta", "alpha beta");
        assert!(new.is_empty());
        assert_eq!(old.len(), 1);
        assert_eq!("alpha new beta"[old[0].0..old[0].1].trim(), "new");
    }

    #[test]
    fn merge_ranges_combines_overlapping_and_adjacent() {
        assert_eq!(
            merge_ranges(vec![(0, 2), (2, 4), (6, 8)]),
            vec![(0, 4), (6, 8)]
        );
        assert_eq!(merge_ranges(vec![(1, 3), (5, 7)]), vec![(1, 3), (5, 7)]);
        assert_eq!(merge_ranges(vec![(0, 0), (2, 2)]), vec![]);
    }
}
