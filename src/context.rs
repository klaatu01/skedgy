pub trait SkedgyContext: Clone {}

impl<T: Clone> SkedgyContext for T {}
