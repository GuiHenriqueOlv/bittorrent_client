use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashSet;
use std::time::Duration;

#[derive(Clone)]
pub struct Tracker {
    peers: Arc<Mutex<HashSet<String>>>,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Inicia o servidor tracker
    pub async fn start(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        println!("Tracker rodando na porta {}", port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let peers = Arc::clone(&self.peers);

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                if let Ok(n) = socket.read(&mut buffer).await {
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();

                    if request.starts_with("REGISTER") {
                        let peer_info = request[9..].to_string(); 
                        peers.lock().await.insert(peer_info.clone());
                        println!("Peer registrado: {}", peer_info);
                    }

                    if request.starts_with("GET_PEERS") {
                        let peer_list: Vec<String> = peers.lock().await.iter().cloned().collect();
                        let response = peer_list.join(",");
                        socket.write_all(response.as_bytes()).await.unwrap();
                    }

                    if request.starts_with("UNREGISTER") {
                        let peer_info = request[11..].to_string();
                        peers.lock().await.remove(&peer_info);
                        println!("Peer removido: {}", peer_info);
                    }
                }
            });
        }
    }

    /// Função para limpar a lista de peers desconectados periodicamente
    pub async fn clean_up(&self) {
        let peers = Arc::clone(&self.peers);
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await; // Limpa a cada 10 segundos (ajuste conforme necessário)
            let mut peers_locked = peers.lock().await;
            peers_locked.retain(|peer| {
                // Aqui você pode adicionar lógica para verificar se o peer está desconectado
                // Exemplo: fazer uma tentativa de ping no peer ou outro critério
                true // Para o exemplo, retém todos, ajuste conforme necessário
            });
        }
    }
}
