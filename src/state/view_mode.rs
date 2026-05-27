#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    Continuous,
    #[default]
    SingleFile,
}
