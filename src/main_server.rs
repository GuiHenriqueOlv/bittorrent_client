mod peer;
use crate::peer::Peer;
use tokio::sync::Mutex; // Para sincronização
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Iniciando servidor BitTorrent...");

    let server_peer = Peer {
        ip: "127.0.0.1".to_string(),
        port: 6882,
        shared_files: vec!["file1.txt".to_string(), "file2.txt".to_string()],
        pending_blocks: Arc::new(Mutex::new(HashSet::new())),
        block_queue: Arc::new(Mutex::new(VecDeque::new())),
        total_blocks: 100,
    };

    if let Err(e) = server_peer.start_server().await {
        eprintln!("Erro ao iniciar o servidor do peer: {}", e);
    }
}
