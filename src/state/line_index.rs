use crate::git::repository::DiffHunk;

const HUNK_HEADER_ROWS: usize = 1;

#[derive(Debug)]
pub struct LineIndex {
    pub hunk_starts: Vec<usize>,
    pub total_rows: usize,
}

impl LineIndex {
    pub fn new(hunks: &[DiffHunk]) -> Self {
        let mut hunk_starts = Vec::with_capacity(hunks.len());
        let mut offset = 0;
        for hunk in hunks {
            hunk_starts.push(offset);
            offset += HUNK_HEADER_ROWS + hunk.lines.len();
        }
        Self {
            hunk_starts,
            total_rows: offset,
        }
    }

    pub fn lookup(&self, global_row: usize) -> Option<(usize, usize)> {
        if global_row >= self.total_rows || self.hunk_starts.is_empty() {
            return None;
        }

        let hunk_idx = self
            .hunk_starts
            .partition_point(|&s| s <= global_row)
            .checked_sub(1)?;
        Some((hunk_idx, global_row - self.hunk_starts[hunk_idx]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repository::{DiffHunk, DiffLine};

    fn line(content: &str) -> DiffLine {
        DiffLine {
            old_lineno: None,
            new_lineno: None,
            origin: ' ',
            content: content.to_string(),
        }
    }

    fn hunk_with_lines(count: usize) -> DiffHunk {
        DiffHunk {
            header: "@@ -1,1 +1,1 @@".to_string(),
            lines: (0..count).map(|i| line(&format!("line {i}"))).collect(),
            insertions: count,
            deletions: 0,
        }
    }

    #[test]
    fn empty_hunks() {
        let index = LineIndex::new(&[]);
        assert_eq!(
            index.total_rows, 0,
            "empty diff should have zero total rows"
        );
        assert_eq!(
            index.lookup(0),
            None,
            "lookup on empty diff should return None"
        );
        assert_eq!(
            index.lookup(999),
            None,
            "out-of-bounds on empty diff should return None"
        );
    }

    #[test]
    fn single_hunk_no_lines() {
        const HEADER_COUNT: usize = 1;
        const HUNK_HEADER: (usize, usize) = (0, 0);

        let hunks = vec![DiffHunk {
            header: "@@ -1,1 +1,1 @@".to_string(),
            lines: vec![],
            insertions: 0,
            deletions: 0,
        }];
        let index = LineIndex::new(&hunks);

        assert_eq!(
            index.total_rows, HEADER_COUNT,
            "header-only hunk should have {HEADER_COUNT} total row"
        );
        assert_eq!(
            index.lookup(0),
            Some(HUNK_HEADER),
            "row 0 should be the hunk header"
        );
        assert_eq!(
            index.lookup(1),
            None,
            "row 1 should be out of bounds for header-only hunk"
        );
    }

    #[test]
    fn single_hunk_three_lines() {
        const HEADER_WITH_LINES: usize = 4;
        const HUNK_HEADER: (usize, usize) = (0, 0);
        const HUNK_0_LINE_0: (usize, usize) = (0, 1);
        const HUNK_0_LINE_1: (usize, usize) = (0, 2);
        const HUNK_0_LINE_2: (usize, usize) = (0, 3);

        let hunks = vec![hunk_with_lines(3)];
        let index = LineIndex::new(&hunks);

        assert_eq!(
            index.total_rows, HEADER_WITH_LINES,
            "header + 3 lines should have {HEADER_WITH_LINES} total rows"
        );
        assert_eq!(
            index.lookup(0),
            Some(HUNK_HEADER),
            "row 0 should be the hunk header"
        );
        assert_eq!(
            index.lookup(1),
            Some(HUNK_0_LINE_0),
            "row 1 should be hunk 0, line 0"
        );
        assert_eq!(
            index.lookup(2),
            Some(HUNK_0_LINE_1),
            "row 2 should be hunk 0, line 1"
        );
        assert_eq!(
            index.lookup(3),
            Some(HUNK_0_LINE_2),
            "row 3 should be hunk 0, line 2"
        );
        assert_eq!(index.lookup(4), None, "row 4 should be out of bounds");
    }

    #[test]
    fn multiple_hunks() {
        const TOTAL_ROWS: usize = 7;
        const HUNK_0_LAST_ROW: (usize, usize) = (0, 2);
        const HUNK_1_HEADER: (usize, usize) = (1, 0);
        const HUNK_1_MIDDLE: (usize, usize) = (1, 2);

        // Hunk 0: header + 2 lines → rows 0, 1, 2
        // Hunk 1: header + 3 lines → rows 3, 4, 5, 6
        let hunks = vec![hunk_with_lines(2), hunk_with_lines(3)];
        let index = LineIndex::new(&hunks);

        assert_eq!(
            index.total_rows, TOTAL_ROWS,
            "two hunks with 2 and 3 lines should have {TOTAL_ROWS} total rows"
        );
        assert_eq!(
            index.lookup(2),
            Some(HUNK_0_LAST_ROW),
            "row 2 should be the last line of hunk 0"
        );
        assert_eq!(
            index.lookup(3),
            Some(HUNK_1_HEADER),
            "row 3 should be the header of hunk 1"
        );
        assert_eq!(
            index.lookup(5),
            Some(HUNK_1_MIDDLE),
            "row 5 should be a middle line of hunk 1"
        );
    }

    #[test]
    fn out_of_bounds() {
        const PAST_THE_END: usize = 2;
        const FAR_PAST_THE_END: usize = 1000;

        // Hunk 0: header + 1 lines -> rows 0, 1
        let hunks = vec![hunk_with_lines(1)];
        let index = LineIndex::new(&hunks);

        assert_eq!(
            index.lookup(PAST_THE_END),
            None,
            "row just past the end should return None"
        );
        assert_eq!(
            index.lookup(FAR_PAST_THE_END),
            None,
            "row far past the end should return None"
        );
    }
}
