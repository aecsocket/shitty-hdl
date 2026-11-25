pub trait Domain: Clone + Copy {}

#[derive(Debug, Clone, Copy)]
pub struct Clock(());
impl Domain for Clock {}
