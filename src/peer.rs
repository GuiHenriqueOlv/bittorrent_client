use tokio::sync::{Mutex, mpsc};
use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File; // Alterado para tokio::fs::File
use std::sync::Arc;
use std::collections::{HashSet, VecDeque};
use std::fs::read_dir; // Para listar arquivos locais




#[derive(Clone)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
    pub shared_files: Vec<String>,
    pub pending_blocks: Arc<Mutex<HashSet<usize>>>, // Use tokio::sync::Mutex
    pub block_queue: Arc<Mutex<VecDeque<usize>>>,   // Use tokio::sync::Mutex
    pub total_blocks: usize,
}

impl Peer {
    // Função para listar arquivos compartilhados por um peer
    pub fn list_shared_files(&self) -> String {
        self.shared_files.join(", ")
    }

    // Função para iniciar o servidor do peer
    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).await?;
        println!("Servidor iniciado no peer: {}:{}", self.ip, self.port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let shared_files = self.shared_files.clone();

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                if let Ok(n) = socket.read(&mut buffer).await {
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();

                    // Verificar qual solicitação foi feita
                    if request.starts_with("LIST_FILES") {
                        let file_list = shared_files.join(", ");
                        socket.write_all(file_list.as_bytes()).await.unwrap();
                    }
                    // Resposta de solicitação de arquivo
                    if request.starts_with("REQUEST_FILE") {
                        let filename = &request[13..]; // Cortar "REQUEST_FILE" da string
                        if let Some(file_path) = shared_files.iter().find(|f| f.as_str() == filename) {
                            if let Ok(mut file) = File::open(file_path).await {  // Não precisa de clone
                                let mut file_buffer = vec![0; 1024];
                                while let Ok(bytes_read) = file.read(&mut file_buffer).await {
                                    if bytes_read == 0 {
                                        break;
                                    }
                                    socket.write_all(&file_buffer[..bytes_read]).await.unwrap();
                                }
                            }
                        }                        
                    }
                }
            });
        }
    }

    // Função para conectar-se a outro peer
    pub async fn connect_to_peer(&self, peer_ip: &str, peer_port: u16) -> Result<TcpStream, Box<dyn std::error::Error>> {
        let address = format!("{}:{}", peer_ip, peer_port);
        let stream = TcpStream::connect(address).await?;
        println!("Conectado ao peer: {}:{}", peer_ip, peer_port);
        Ok(stream)
    }

    // Função para buscar arquivos compartilhados em outro peer
    pub async fn search_files(&self, peer_ip: &str, peer_port: u16) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut stream = self.connect_to_peer(peer_ip, peer_port).await?;

        // Solicitar lista de arquivos
        stream.write_all(b"LIST_FILES").await?;

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let file_list = String::from_utf8_lossy(&buffer[..n]).to_string();

        let files = file_list.split(", ").map(|s| s.to_string()).collect::<Vec<String>>();
        Ok(files)
    }

    // Função para baixar um arquivo de outro peer
    pub async fn download_file(&self, peer_ip: &str, peer_port: u16, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = self.connect_to_peer(peer_ip, peer_port).await?;

        // Solicitar o arquivo
        let request = format!("REQUEST_FILE {}", filename);
        stream.write_all(request.as_bytes()).await?;

        let mut buffer = vec![0; 1024];
        let mut file = File::create(filename).await?;
        while let Ok(n) = stream.read(&mut buffer).await {
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n]).await?;
        }

        println!("Arquivo {} baixado com sucesso!", filename);
        Ok(())
    }
}

pub fn list_local_files(directory: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = read_dir(directory) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                files.push(file_name);
            }
        }
    }
    files
}