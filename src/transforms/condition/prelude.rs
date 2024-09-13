pub trait Builder {
    type Output;

    fn build(self) -> Self::Output;
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct WrappedConfig<T> {
    pub value: T,
}

impl<Cond, T: Builder<Output = Cond>> Builder for WrappedConfig<T> {
    type Output = Cond;

    fn build(self) -> Self::Output {
        self.value.build()
    }
}

#[enum_dispatch::enum_dispatch]
pub trait Evaluate {
    fn evaluate(&self, event: &crate::event::Event) -> bool;
}
