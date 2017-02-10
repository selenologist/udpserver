use std::sync::mpsc::*;
use std::sync::{Arc, Mutex};

pub enum BroadcastEnum{
    Datagram(Vec<u8>)
}

pub type BroadcastMessage = Arc<BroadcastEnum>;
pub struct BroadcastClientList(Arc<Mutex<Vec<Sender<BroadcastMessage>>>>);

impl BroadcastClientList{
    pub fn new() -> Self{
        BroadcastClientList(Arc::new(Mutex::new(Vec::new())))
    }
    pub fn new_client(&self) -> Receiver<BroadcastMessage>{ // blocking lock
        use std::ops::DerefMut;
        let &BroadcastClientList(ref inner) = self;
        match inner.lock(){
            Ok(mut guard) => {
                let mut list = guard.deref_mut();
                let (tx, rx) = channel();
                list.push(tx);
                return rx;
            },
            Err(p) => {
                panic!("Broadcast client list poisoned when trying to add new client: {:?}", p);
            }
        }
    }
}

impl Clone for BroadcastClientList{
    fn clone(&self) -> Self{
        let &BroadcastClientList(ref inner) = self;
        BroadcastClientList(inner.clone())
    }
}

pub struct Broadcaster{
    rx: Receiver<BroadcastMessage>, // tx end stored in Server and passed to Clients

    client_tx_mutexed: BroadcastClientList // this is an Arc<Mutex> pushed to and swept in Server, written in Broadcaster
}

impl Broadcaster{
    pub fn new(rx: Receiver<BroadcastMessage>, client_tx_mutexed: BroadcastClientList) -> Broadcaster{
        Broadcaster{
            rx: rx,
            client_tx_mutexed: client_tx_mutexed
        }
    }

    pub fn run(&mut self){
        loop{
            for in_msg in self.rx.iter(){ // read a message on the broadcast receive channel
                let &BroadcastClientList(ref inner) = &self.client_tx_mutexed;
                match inner.lock(){ // block until we successfully lock the client list
                    Ok(ref guard) =>
                        {
                            let ref client_tx = *guard;
                            for client in client_tx.iter(){ // send the message to every client
                                match client.send(in_msg.clone()){
                                    Ok(_) => (),
                                    Err(e) => {
                                        println!("Error in broadcast thread: {:?}", e);
                                        ()
                                    }
                                }
                            }
                        },
                    Err(p) => {
                        println!("Broadcast client list poisoned while trying to read in Broadcaster: {:?}", p);
                    }
                }
            }
        }
    }
}
