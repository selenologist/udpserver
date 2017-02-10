use std::sync::mpsc::*;
use std::sync::Arc;
use std::mem;
use std::net::{UdpSocket, SocketAddr};
use std::thread;
use std::collections::HashMap;

use broadcast::*;
use client::*;

enum ServerBroadcaster{
    NotStarted(Broadcaster),
    Started(thread::JoinHandle<()>),
    Limbo // needed for the brief period when there is no thread nor broadcaster available to Server.run()
}

enum ServerTXThread{
    NotStarted(Receiver<OutgoingClientMessage>), // tx thread needs to listen to this to send UDP messages out
    Started(thread::JoinHandle<()>),             // but after that, the server can't have the receiver
    Limbo,
}

pub struct Server{
    broadcast_clients: BroadcastClientList,
    client_tx_list:    HashMap<SocketAddr, Sender<ClientRXMessage>>,
    tx:                Sender<OutgoingClientMessage>, // to be cloned to clients
    broadcast_tx:      Sender<BroadcastMessage>,
    broadcaster:       ServerBroadcaster,   // this is initialised but not run (in own thread) until Server.run()
                                            // where it must then be std::mem::replaced with None
                                            // in Server and Broadcaster moved into its own thread
                                            // where it can run itself
    
    udp_socket:        UdpSocket,
    tx_thread:         ServerTXThread, // blocks on server receiver, sending out new UDP messages
}

impl Server{
    pub fn new(addr: (&str, u16)) -> Server{
        let socket = UdpSocket::bind(addr).expect("Failed to bind UDP socket");
        let bcl = BroadcastClientList::new();
        let (tx, rx) = channel();
        let (broadcast_tx, broadcast_rx) = channel();

        let broadcaster = Broadcaster::new(broadcast_rx, bcl.clone());

        Server{
            broadcast_clients: bcl,
            client_tx_list:    HashMap::new(),
            tx:                tx,
            broadcast_tx:      broadcast_tx,
            broadcaster:       ServerBroadcaster::NotStarted(broadcaster),
            udp_socket:        socket,
            tx_thread:         ServerTXThread::NotStarted(rx),
        }
    }

    fn start_broadcaster(&mut self){
        let mut broadcaster = {
            if let ServerBroadcaster::NotStarted(b) = mem::replace(&mut self.broadcaster, ServerBroadcaster::Limbo){
                b
            }
            else{
                panic!("Somehow got in Server.run() with a broadcaster that had already started or was in Limbo");
            }
        };

        self.broadcaster = ServerBroadcaster::Started(thread::Builder::new().name("broadcast".to_string()).spawn(move ||
        {
            println!("Broadcaster thread running.");
            broadcaster.run();
        }).expect("Failed to create Broadcaster thread"));
    }

    fn start_tx_thread(&mut self){
        use std::os::unix::io::{AsRawFd,FromRawFd};

        let rx = {
            if let ServerTXThread::NotStarted(r) = mem::replace(&mut self.tx_thread, ServerTXThread::Limbo){
                r
            }
            else{
                panic!("Somehow got in Server.run() with a TX thread that had already been started or was in Limbo");
            }};

        let tx_socket = unsafe{
            // duplicate the socket by using the same fd. This should be safe.
            // but to rust standards, it is not because I could supply any fd.

            UdpSocket::from_raw_fd(self.udp_socket.as_raw_fd())
        };

        self.tx_thread = ServerTXThread::Started(
                thread::Builder::new()
                .name("UDP tx".to_string())
                .spawn(move ||
        {
            println!("UDP tx thread running.");
            loop{
                for outgoing in rx.iter(){
                    match outgoing.message{
                        ClientTXMessage::Datagram(ref v) =>
                            match tx_socket.send_to(v.as_slice(), &outgoing.addr){
                                Ok(sent) => {
                                    if sent != v.len(){
                                        println!("Short write to {:?}; only {}, expected {}",
                                                 outgoing.addr, sent, v.len());
                                    }
                                },
                                Err(e) => {
                                    println!("Error writing UDP to {:?}, {:?}",
                                             outgoing.addr, e);
                                }
                            }
                    }
                }
            }
        }).expect("Failed to make the UDP tx thread"));
    }


    pub fn run(&mut self){
        self.start_broadcaster();

        self.start_tx_thread();

        let mut client_threads = Vec::new();

        loop{
            const MAX_UDP_RECV:usize = 1400;
            let mut buf = [0u8;MAX_UDP_RECV]; 
            match self.udp_socket.recv_from(&mut buf){ // this can and will block!
                Ok((amt, src)) => {
                    let lookup_result:bool = {
                        match self.client_tx_list.get(&src){ // borrow hack
                            Some(ref client) => {
                                match client.send(ClientRXMessage::Datagram(
                                                  buf[0..amt].to_vec())){
                                    Ok(_)  => (),
                                    Err(e) => {
                                        println!("Error adding message from {:?} to broadcast channel: {:?}", src, e);
                                        ()
                                    }
                                };
                                true
                            },
                            None => false
                        }};

                    if lookup_result == false{
                        // Unknown UDP packet? Register a new client! What could go wrong?
                        // No seriously this is absolutely terrible, you can use this server to DoS
                        // This is for testing only

                        println!("New client {:?}", src);
                        let broadcast_rx = self.broadcast_clients.new_client();
                        let (mut client, client_tx) = Client::new(src.clone(),
                                                              self.tx.clone(),
                                                              self.broadcast_tx.clone(),
                                                              broadcast_rx);
                        self.client_tx_list.insert(src.clone(), client_tx);

                        client_threads.push(
                            thread::Builder::new()
                            .name(format!("cli{:?}", src))
                            .spawn(move ||{
                                client.run()
                            })
                            .expect("Failed to make a client thread"));
                    }
                }
                Err(e) => {
                    println!("Error receiving from UDP socket: {:?}", e);
                }
            }
        }
    }
}

    






        





    
