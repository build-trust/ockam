// use crate::message::RouterMessage;
// use tokio::sync::mpsc;
// use tokio::sync::mpsc::{Receiver, Sender};
//
// pub struct TransportRouter {
//     map: hashbrown::HashMap<Vec<u8>, Sender<RouterMessage>>,
//     tx: Sender<RouterMessage>,
//     rx: Receiver<RouterMessage>,
// }
//
// impl TransportRouter {
//     pub fn new() -> Self {
//         let (tx, mut rx) = mpsc::channel(10);
//         let map = hashbrown::HashMap::new();
//         TransportRouter { map, tx, rx }
//     }
//
//     pub fn get_sender(&self) -> Sender<RouterMessage> {
//         self.tx.clone()
//     }
//
//     pub fn register(&mut self, address: Vec<u8>, sender: Sender<RouterMessage>) {
//         self.map.insert(address, sender);
//     }
//
//     pub async fn route(&self, msg: RouterMessage) -> Result<(), String> {
//         return match self.map.get(&msg.onward_route.addrs[0].address[0..]) {
//             Some(t) => {
//                 t.send(msg).await.unwrap();
//                 Ok(())
//             }
//             None => Err(String::from("address not found")),
//         };
//     }
//
//     pub async fn stop(&mut self) {
//         loop {
//             match self.rx.recv().await {
//                 Some(m) => match self.map.get(&m.onward_route.addrs[0].address[0..]) {
//                     Some(t) => {
//                         t.send(m).await.unwrap();
//                     }
//                     None => {
//                         println!("no such address {:?}!", m.onward_route.addrs[0].address);
//                         return;
//                     }
//                 },
//                 None => {
//                     println!("no message!");
//                     return;
//                 }
//             }
//         }
//     }
// }
