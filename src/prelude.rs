pub type Sender = tokio::sync::mpsc::Sender<crate::event::Event>;
pub type Receiver = tokio::sync::mpsc::Receiver<crate::event::Event>;

#[inline]
pub fn create_channel(size: usize) -> (Sender, Receiver) {
    tokio::sync::mpsc::channel(size)
}
