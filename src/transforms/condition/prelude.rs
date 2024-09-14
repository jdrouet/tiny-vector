pub trait Builder {
    type Output;
    type Error: Into<super::BuildError>;

    fn build(self) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct WrappedConfig<T> {
    pub value: T,
}

impl<Cond, Err, T> Builder for WrappedConfig<T>
where
    Err: Into<super::BuildError>,
    T: Builder<Output = Cond, Error = Err>,
{
    type Output = Cond;
    type Error = Err;

    fn build(self) -> Result<Self::Output, Self::Error> {
        self.value.build()
    }
}

#[enum_dispatch::enum_dispatch]
pub trait Evaluate {
    fn evaluate(&self, event: &crate::event::Event) -> bool;
}
