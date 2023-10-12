pub enum DeleteMode {
    All,
    Selected(Vec<String>),
    Single(String),
    Default,
}
