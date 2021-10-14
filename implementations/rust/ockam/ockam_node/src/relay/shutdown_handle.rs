// use crate::error::Error;
// use crate::tokio::sync::oneshot;
// use ockam_core::Result;

// pub struct ShutdownListener {
//     rx_shutdown: oneshot::Receiver<()>,
//     tx_ack: oneshot::Sender<()>,
// }

// impl ShutdownListener {
//     pub fn consume(self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
//         (self.rx_shutdown, self.tx_ack)
//     }
// }

// #[derive(Debug)]
// pub struct ShutdownHandle {
//     tx_shutdown: oneshot::Sender<()>,
//     rx_ack: oneshot::Receiver<()>,
// }

// impl ShutdownHandle {
//     pub fn create() -> (ShutdownHandle, ShutdownListener) {
//         let (tx_shutdown, rx_shutdown) = oneshot::channel();
//         let (tx_ack, rx_ack) = oneshot::channel();

//         (
//             ShutdownHandle {
//                 tx_shutdown,
//                 rx_ack,
//             },
//             ShutdownListener {
//                 rx_shutdown,
//                 tx_ack,
//             },
//         )
//     }
// }

// impl ShutdownHandle {
//     pub async fn shutdown(self) -> Result<()> {
//         // Ignore error
//         let _ = self.tx_shutdown.send(());
//         self.rx_ack.await.map_err(|_| Error::ShutdownAckError)?;

//         Ok(())
//     }
// }
