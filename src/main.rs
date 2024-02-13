use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum State {
    Init,
    Running,
    Stopped,
}

#[derive(Serialize, Deserialize, Debug)]
enum MessageType {
    Proposal,
    Acknowledgment,
    Commit,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    sender_id: u64,
    message_type: MessageType,
    proposed_state: State,
    proposal_id: String,
}

struct Node {
    id: u64,
    state: Arc<Mutex<State>>,
    peers: HashMap<u64, String>, // Map of peer IDs and their addresses
    address: String, // Address to listen on
    tx: mpsc::Sender<Message>,
    proposal_acknowledgments: Arc<Mutex<HashMap<String, HashSet<u64>>>>, // Tracks acknowledgments for each proposal
}

impl Node {
    async fn send_message(&self, message: &Message, receiver_address: &str) -> io::Result<()> {
        let mut stream = TcpStream::connect(receiver_address).await?;
        let message = serde_json::to_vec(message)?;
        stream.write_all(&message).await?;
        Ok(())
    }

    async fn broadcast_proposal(&self, proposed_state: State) {
        let proposal_id = Uuid::new_v4().to_string(); // Using UUID as a unique identifier for the proposal
        let message = Message {
            sender_id: self.id,
            message_type: MessageType::Proposal,
            proposed_state,
            proposal_id: proposal_id.clone(),
        };

        {
            let mut proposal_acknowledgments = self.proposal_acknowledgments.lock().await;
            proposal_acknowledgments.insert(proposal_id.clone(), HashSet::new());
        }

        for address in self.peers.values() {
            if let Err(e) = self.send_message(&message, address).await {
                eprintln!("Failed to send message to {}: {:?}", address, e);
            }
        }

        // Wait for acknowledgments
        self.wait_for_acknowledgments(proposal_id).await;
    }

    async fn wait_for_acknowledgments(&self, proposal_id: String) {
        let majority = (self.peers.len() / 2) + 1; // Simple majority

        loop {
            let ack_count = {
                let acks = self.proposal_acknowledgments.lock().await;
                acks.get(&proposal_id)
                    .map(|acks| acks.len())
                    .unwrap_or(0)
            };

            if ack_count >= majority {
                // Commit the proposal
                let commit_message = Message {
                    sender_id: self.id,
                    message_type: MessageType::Commit,
                    proposed_state: State::Running, // This should be the state you proposed earlier
                    proposal_id: proposal_id.clone(),
                };

                for address in self.peers.values() {
                    self.send_message(&commit_message, address).await.unwrap();
                }

                println!("Node {} committed the proposal: {}", self.id, proposal_id);
                break;
            }

            // Sleep for a short duration before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn handle_incoming_messages(&self, mut rx: mpsc::Receiver<Message>) {
        while let Some(message) = rx.recv().await {
            match message.message_type {
                MessageType::Proposal => {
                    // Attempt to get the sender's address from the peers map
                    if let Some(sender_address) = self.peers.get(&message.sender_id) {
                        let ack_message = Message {
                            sender_id: self.id,
                            message_type: MessageType::Acknowledgment,
                            proposed_state: message.proposed_state.clone(),
                            proposal_id: message.proposal_id.clone(),
                        };
                        self.send_message(&ack_message, sender_address).await.unwrap(); // Consider handling this unwrap as well
                    } else {
                        // Log a warning or handle the missing address case as needed
                        eprintln!("Warning: Unknown sender_id {}, skipping.", message.sender_id);
                    }
                }
                MessageType::Acknowledgment => {
                    if let Some(acks) = self.proposal_acknowledgments.lock().await.get_mut(&message.proposal_id) {
                        acks.insert(message.sender_id);
                    }
                }
                MessageType::Commit => {
                    let mut state = self.state.lock().await;
                    *state = message.proposed_state.clone();
                    println!("Node {} committed to state {:?}", self.id, *state);
                }
                _ => {}
            }
        }
    }

    async fn listen(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address).await?;
        println!("Node {} listening on {}", self.id, self.address);

        loop {
            let (mut socket, _) = listener.accept().await?;

            let tx = self.tx.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    match socket.read(&mut buf).await {
                        Ok(0) => {
                            println!("Connection closed");
                            break; // Connection was closed
                        }
                        Ok(n) => {
                            if let Ok(message) = serde_json::from_slice::<Message>(&buf[..n]) {
                                tx.send(message).await.expect("Failed to send message to channel");
                            } else {
                                println!("Failed to deserialize message");
                            }
                        }
                        Err(e) => {
                            println!("Failed to read from socket: {:?}", e);
                            break;
                        }
                    }
                }
            });
        }
    }
}

async fn simulate_client() -> io::Result<()> {
    let mut stream = TcpStream::connect("0.0.0.0:8080").await?;
    let message = Message {
        sender_id: 999,
        message_type: MessageType::Proposal,
        proposed_state: State::Running,
        proposal_id: Uuid::new_v4().to_string(),
    };
    let serialized_message = serde_json::to_vec(&message)?;
    stream.write_all(&serialized_message).await?;
    stream.flush().await?; // Ensure the message is sent immediately
    Ok(())
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(State::Init));
    let proposal_acknowledgments = Arc::new(Mutex::new(HashMap::new()));

    let (tx1, rx1) = mpsc::channel(32);
    let node1 = Arc::new(Node {
        id: 1,
        state: state.clone(),
        peers: HashMap::from([(2, "0.0.0.0:8081".to_string())]),
        address: "0.0.0.0:8080".to_string(),
        tx: tx1,
        proposal_acknowledgments: proposal_acknowledgments.clone(),
    });

    let (tx2, rx2) = mpsc::channel(32);
    let node2 = Arc::new(Node {
        id: 2,
        state: state.clone(),
        peers: HashMap::from([(1, "0.0.0.0:8080".to_string())]),
        address: "0.0.0.0:8081".to_string(),
        tx: tx2,
        proposal_acknowledgments,
    });

    let node1_clone_for_messages = Arc::clone(&node1);
    tokio::spawn(async move {
        node1_clone_for_messages.handle_incoming_messages(rx1).await;
    });

    let node2_clone_for_messages = Arc::clone(&node2);
    tokio::spawn(async move {
        node2_clone_for_messages.handle_incoming_messages(rx2).await;
    });

    // Listen for incoming connections
    let node1_clone_for_listen = Arc::clone(&node1);
    tokio::spawn(async move {
        node1_clone_for_listen.listen().await.expect("Node 1 failed to listen");
    });

    let node2_clone_for_listen = Arc::clone(&node2);
    tokio::spawn(async move {
        node2_clone_for_listen.listen().await.expect("Node 2 failed to listen");
    });

    // Ensure the servers have time to start up
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Use the original `node1` Arc to broadcast a proposal
    node1.broadcast_proposal(State::Running).await;

    // Start the simulation after a short delay to ensure nodes are listening
    tokio::time::sleep(Duration::from_secs(2)).await;
    if let Err(e) = simulate_client().await {
        eprintln!("Failed to simulate client: {:?}", e);
    }
}
