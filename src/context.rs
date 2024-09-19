pub trait SkedgyContext: Clone + Send + Sync + 'static {}

impl<T: Clone + Send + Sync + 'static> SkedgyContext for T {}
