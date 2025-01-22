mod peer;
use crate::peer::Peer;
use tokio::sync::Mutex; // Para sincronização
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Iniciando cliente BitTorrent...");

    let client_peer = Peer {
        ip: "127.0.0.1".to_string(),
        port: 6883,
        shared_files: vec![],
        pending_blocks: Arc::new(Mutex::new(HashSet::new())),
        block_queue: Arc::new(Mutex::new(VecDeque::new())),
        total_blocks: 100,
    };

    // Aguarde o servidor começar
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    match client_peer.search_files("127.0.0.1", 6882).await {
        Ok(files) => {
            println!("Arquivos disponíveis no peer remoto: {:?}", files);
            if let Some(filename) = files.get(0) {
                // Baixar o primeiro arquivo
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
