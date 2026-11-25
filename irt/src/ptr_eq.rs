#[macro_export]
macro_rules! ptr_identity {
    ($vis:vis struct $wrap:ident<$lt:lifetime>($imp:ty);) => {
        #[derive(Debug, Clone, Copy)]
        $vis struct $wrap<$lt>(pub &$lt $imp);

        impl std::cmp::PartialEq for $wrap<'_> {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self.0, other.0)
            }
        }

        impl std::cmp::Eq for $wrap<'_> {}

        impl std::hash::Hash for $wrap<'_> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::ptr::hash(self.0, state);
            }
        }
    };
}
