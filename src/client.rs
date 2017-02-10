use std::net::SocketAddr;
use std::sync::mpsc::*;
use std::sync::Arc;

use broadcast::*;

#[derive(Clone)]
pub enum ClientRXMessage{
    Datagram(Vec<u8>),
    ShouldClose,
}

#[derive(Clone)]
pub enum ClientTXMessage{
    Datagram(Vec<u8>)
}

pub struct OutgoingClientMessage{
    pub addr:    SocketAddr,
    pub message: ClientTXMessage
}

pub struct Client{
    addr: SocketAddr,
    out_tx: Sender<OutgoingClientMessage>,   // Outgoing messages go over this Sender
    rx: Receiver<ClientRXMessage>,           // Incoming specific messages thru this Receiver
    broadcast_tx: Sender<BroadcastMessage>,  // Outgoing broadcast messages over this Sender
    broadcast_rx: Receiver<BroadcastMessage> // Incoming broadcast messages thru this receiver
}

impl Client{
    pub fn new(addr: SocketAddr, out_tx: Sender<OutgoingClientMessage>, broadcast_tx: Sender<BroadcastMessage>, broadcast_rx: Receiver<BroadcastMessage>) -> (Client,Sender<ClientRXMessage>){
        let (tx, rx) = channel();
        let cli = Client{
            addr:         addr,
            out_tx:       out_tx,
            rx:           rx,
            broadcast_tx: broadcast_tx,
            broadcast_rx: broadcast_rx
        };

        (cli, tx)
    }

    pub fn run(&mut self){
        loop{
        let (ref broadcast_rx, ref rx) = (&self.broadcast_rx, &self.rx);
        select!{
            bmsg = broadcast_rx.recv() =>
                {match *(bmsg.expect("Failed to unwrap broadcast message")){
                    BroadcastEnum::Datagram(ref v) => {
                        match self.out_tx.send(OutgoingClientMessage{
                                               addr: self.addr.clone(),
                                               message: ClientTXMessage::Datagram(v.clone())
                                           }){
                            Ok(_)  => (),
                            Err(e) => {
                                println!("Error {:?} on client {:?} outgoing main message send",
                                         e, self.addr);
                                ()
                            }
                        }
                    },
                }},
            rmsg = rx.recv() =>
                {match rmsg.expect("Failed to unwrap general message"){
                    ClientRXMessage::ShouldClose =>
                        {println!("Client {:?} closing on channel request",
                                  self.addr);
                         return; },
                    ClientRXMessage::Datagram(ref v) => {
                        match self.broadcast_tx.send(
                            Arc::new(BroadcastEnum::Datagram(v.clone()))){
                                Ok(_) => (),
                                Err(e) => {
                                    println!("Error {:?} on client {:?} outgoing broadcast message send",
                                             e, self.addr);
                                    ()
                                }
                            }
                        }
                    }   
                }
        }
        }
    }
}


