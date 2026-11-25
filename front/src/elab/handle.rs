use super::DesignId;

macro_rules! handle {
    ($($vis:vis $name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            $vis struct $name {
                pub(crate) design_id: DesignId,
                pub(crate) index: usize,
            }
        )*
    };
}

handle!(
    pub ExprHandle,
    pub RegisterHandle,
    pub ActionHandle,
    pub InterfaceHandle,
    pub ClockHandle,
    pub BranchHandle,
);
