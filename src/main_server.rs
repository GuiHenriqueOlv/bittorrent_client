mod peer;
use crate::peer::{Peer, list_local_files};
use tokio::sync::{Mutex}; // Alterado para usar tokio::sync::Mutex
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Iniciando cliente BitTorrent...");

    // Peer servidor compartilhando arquivos
    let server_peer = Peer {
        ip: "127.0.0.1".to_string(),
        port: 6882,
        shared_files: vec!["file1.txt".to_string(), "file2.txt".to_string()],
        pending_blocks: Arc::new(Mutex::new(HashSet::new())),
        block_queue: Arc::new(Mutex::new(VecDeque::new())),
        total_blocks: 100,
    };

    let peer = Peer {
        ip: "127.0.0.1".to_string(),
        port: 6882,
        shared_files: list_local_files("shared_files"), // Passando o diretório correto
        pending_blocks: Arc::new(Mutex::new(HashSet::new())), // Alterado para usar tokio::sync::Mutex
        block_queue: Arc::new(Mutex::new(VecDeque::new())), // Alterado para usar tokio::sync::Mutex
        total_blocks: 100, // Mantido
    };

    // Iniciar o servidor do peer
    tokio::spawn(async move {
        if let Err(e) = server_peer.start_server().await {
            eprintln!("Erro ao iniciar o servidor do peer: {}", e);
        }
    });

    // Aguarde o servidor começar
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Peer cliente tentando buscar arquivos
    let client_peer = Peer {
        ip: "127.0.0.1".to_string(),
        port: 6883,
        shared_files: vec![], // Cliente não tem arquivos compartilhados
        pending_blocks: Arc::new(Mutex::new(HashSet::new())), // Alterado para usar tokio::sync::Mutex
        block_queue: Arc::new(Mutex::new(VecDeque::new())), // Alterado para usar tokio::sync::Mutex
        total_blocks: 100, // Mantido
    };

    // Buscar arquivos do peer servidor
    match client_peer.search_files("127.0.0.1", 6882).await {
        Ok(files) => {
            println!("Arquivos disponíveis no peer remoto: {:?}", files);
            if let Some(filename) = files.get(0) { // Baixar o primeiro arquivo
                if let Err(e) = client_peer.download_file("127.0.0.1", 6882, filename).await {
                    eprintln!("Erro ao baixar o arquivo: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Erro ao buscar arquivos: {}", e);
        }
    }
}
