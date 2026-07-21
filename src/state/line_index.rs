use crate::git::repository::{DiffSection, DiffSectionKind};

const HUNK_HEADER_ROWS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexRow {
    SectionHeader(usize),
    HunkHeader(usize),
    DiffLine(usize, usize),
}

#[derive(Debug)]
pub struct LineIndex {
    pub hunk_starts: Vec<usize>,
    pub section_header_rows: Vec<(usize, DiffSectionKind)>,
    pub total_rows: usize,
}

impl LineIndex {
    pub fn new(sections: &[DiffSection]) -> Self {
        let total_hunks = sections.iter().map(|s| s.hunks.len()).sum();
        let mut hunk_starts = Vec::with_capacity(total_hunks);
        let mut section_header_rows = Vec::new();
        let show_headers = sections.len() > 1;
        let mut offset = 0;

        for section in sections {
            if show_headers {
                section_header_rows.push((offset, section.kind));
                offset += 1;
            }
            for hunk in &section.hunks {
                hunk_starts.push(offset);
                offset += HUNK_HEADER_ROWS + hunk.lines.len();
            }
        }

        Self {
            hunk_starts,
            section_header_rows,
            total_rows: offset,
        }
    }

    pub fn lookup(&self, global_row: usize) -> Option<IndexRow> {
        if global_row >= self.total_rows {
            return None;
        }

        if let Some(idx) = self
            .section_header_rows
            .iter()
            .position(|&(row, _)| row == global_row)
        {
            return Some(IndexRow::SectionHeader(idx));
        }

        if self.hunk_starts.is_empty() {
            return None;
        }

        let hunk_idx = self
            .hunk_starts
            .partition_point(|&s| s <= global_row)
            .checked_sub(1)?;
        let offset = global_row - self.hunk_starts[hunk_idx];
        if offset == 0 {
            Some(IndexRow::HunkHeader(hunk_idx))
        } else {
            Some(IndexRow::DiffLine(hunk_idx, offset - 1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repository::{DiffHunk, DiffLine, DiffSection, DiffSectionKind};

    fn line(content: &str) -> DiffLine {
        DiffLine {
            old_lineno: None,
            new_lineno: None,
            origin: ' ',
            content: content.to_string(),
        }
    }

    fn hunk_with_lines(count: usize) -> DiffHunk {
        DiffHunk::new(
            "@@ -1,1 +1,1 @@".to_string(),
            (0..count).map(|i| line(&format!("line {i}"))).collect(),
        )
    }

    fn single_section(hunks: Vec<DiffHunk>) -> Vec<DiffSection> {
        vec![DiffSection {
            kind: DiffSectionKind::Staged,
            hunks,
        }]
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

        let sections = single_section(vec![DiffHunk::new("@@ -1,1 +1,1 @@".to_string(), vec![])]);
        let index = LineIndex::new(&sections);

        assert_eq!(
            index.total_rows, HEADER_COUNT,
            "header-only hunk should have {HEADER_COUNT} total row"
        );
        assert_eq!(
            index.lookup(0),
            Some(IndexRow::HunkHeader(0)),
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

        let sections = single_section(vec![hunk_with_lines(3)]);
        let index = LineIndex::new(&sections);

        assert_eq!(
            index.total_rows, HEADER_WITH_LINES,
            "header + 3 lines should have {HEADER_WITH_LINES} total rows"
        );
        assert_eq!(
            index.lookup(0),
            Some(IndexRow::HunkHeader(0)),
            "row 0 should be the hunk header"
        );
        assert_eq!(
            index.lookup(1),
            Some(IndexRow::DiffLine(0, 0)),
            "row 1 should be hunk 0, line 0"
        );
        assert_eq!(
            index.lookup(2),
            Some(IndexRow::DiffLine(0, 1)),
            "row 2 should be hunk 0, line 1"
        );
        assert_eq!(
            index.lookup(3),
            Some(IndexRow::DiffLine(0, 2)),
            "row 3 should be hunk 0, line 2"
        );
        assert_eq!(index.lookup(4), None, "row 4 should be out of bounds");
    }

    #[test]
    fn multiple_hunks() {
        const TOTAL_ROWS: usize = 7;

        // Hunk 0: header + 2 lines → rows 0, 1, 2
        // Hunk 1: header + 3 lines → rows 3, 4, 5, 6
        let sections = single_section(vec![hunk_with_lines(2), hunk_with_lines(3)]);
        let index = LineIndex::new(&sections);

        assert_eq!(
            index.total_rows, TOTAL_ROWS,
            "two hunks with 2 and 3 lines should have {TOTAL_ROWS} total rows"
        );
        assert_eq!(
            index.lookup(2),
            Some(IndexRow::DiffLine(0, 1)),
            "row 2 should be the last line of hunk 0"
        );
        assert_eq!(
            index.lookup(3),
            Some(IndexRow::HunkHeader(1)),
            "row 3 should be the header of hunk 1"
        );
        assert_eq!(
            index.lookup(5),
            Some(IndexRow::DiffLine(1, 1)),
            "row 5 should be a middle line of hunk 1"
        );
    }

    #[test]
    fn out_of_bounds() {
        const PAST_THE_END: usize = 2;
        const FAR_PAST_THE_END: usize = 1000;

        // Hunk 0: header + 1 line -> rows 0, 1
        let sections = single_section(vec![hunk_with_lines(1)]);
        let index = LineIndex::new(&sections);

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

    #[test]
    fn two_sections_insert_headers() {
        // Section 0 (Staged): 1 hunk with 2 lines  → header@0, hunk_header@1, lines@2,3
        // Section 1 (Unstaged): 1 hunk with 1 line → header@4, hunk_header@5, line@6
        let sections = vec![
            DiffSection {
                kind: DiffSectionKind::Staged,
                hunks: vec![hunk_with_lines(2)],
            },
            DiffSection {
                kind: DiffSectionKind::Unstaged,
                hunks: vec![hunk_with_lines(1)],
            },
        ];
        let index = LineIndex::new(&sections);

        assert_eq!(index.total_rows, 7, "two sections should have 7 total rows");
        assert_eq!(
            index.lookup(0),
            Some(IndexRow::SectionHeader(0)),
            "row 0 should be section header 0"
        );
        assert_eq!(
            index.lookup(1),
            Some(IndexRow::HunkHeader(0)),
            "row 1 should be hunk header 0"
        );
        assert_eq!(
            index.lookup(2),
            Some(IndexRow::DiffLine(0, 0)),
            "row 2 should be hunk 0, line 0"
        );
        assert_eq!(
            index.lookup(3),
            Some(IndexRow::DiffLine(0, 1)),
            "row 3 should be hunk 0, line 1"
        );
        assert_eq!(
            index.lookup(4),
            Some(IndexRow::SectionHeader(1)),
            "row 4 should be section header 1"
        );
        assert_eq!(
            index.lookup(5),
            Some(IndexRow::HunkHeader(1)),
            "row 5 should be hunk header 1"
        );
        assert_eq!(
            index.lookup(6),
            Some(IndexRow::DiffLine(1, 0)),
            "row 6 should be hunk 1, line 0"
        );
    }
}
