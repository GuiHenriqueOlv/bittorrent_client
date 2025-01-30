use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::read_dir;
use sha2::{Sha256, Digest};
use hex;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
    pub shared_files: Vec<String>,
    pub name: String,
}

impl Peer {
    pub fn new(ip: String, port: u16, shared_files: Vec<String>, name: String) -> Self {
        Self {
            ip,
            port,
            shared_files,
            name,
        }
    }

    fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub async fn send_file_in_blocks(&self, file_path: &str, socket: &mut TcpStream) -> std::io::Result<()> {
        let mut file = File::open(file_path).await?;
        let mut buffer = vec![0; 1024 * 1024]; // 1MB por bloco
        let mut block_id = 0;

        while let Ok(n) = file.read(&mut buffer).await {
            if n == 0 { break; }
            println!("Lido {} bytes do arquivo original.", n);
            let block = &buffer[..n];
            let checksum = Self::calculate_checksum(block);
            let block_header = format!("BLOCK {} {}", block_id, checksum);
            socket.write_all(block_header.as_bytes()).await?;
            socket.write_all(block).await?;
            println!("Enviado bloco {} com {} bytes.", block_id, n);
            block_id += 1;
        }

        Ok(())
    }

    pub async fn receive_file_in_blocks(&self, file_path: &str, socket: &mut TcpStream) -> std::io::Result<()> {
        let mut file = File::create(file_path).await?;
        let mut buffer = vec![0; 1024 * 1024 + 64]; // Buffer maior para incluir o cabeçalho do bloco

        loop {
            let n = socket.read(&mut buffer).await?;
            if n == 0 { break; }
            println!("Recebido {} bytes do socket.", n);
            let header_data = String::from_utf8_lossy(&buffer[..64]).to_string();
            let header_parts: Vec<&str> = header_data.split_whitespace().collect();
            let block_id = header_parts[1];
            let expected_checksum = header_parts[2];
            let block_data = &buffer[64..n];

            let calculated_checksum = Self::calculate_checksum(block_data);
            if calculated_checksum == expected_checksum {
                println!("Bloco {} válido", block_id);
                file.write_all(block_data).await?;
                println!("Escrito {} bytes no arquivo.", n - 64);
            } else {
                println!("Bloco {} inválido", block_id);
            }
        }

        Ok(())
    }

    pub async fn list_peer_files(&self, peer_addr: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(peer_addr).await?;
        stream.write_all(b"LIST_FILES").await?;
        
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let files = String::from_utf8_lossy(&buffer[..n])
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
            
        Ok(files)
    }

    pub async fn list_network_files(&self, peers: Vec<String>) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let mut network_files = Vec::new();
        
        for peer in peers {
            if peer != format!("{}:{}", self.ip, self.port) {
                match self.list_peer_files(&peer).await {
                    Ok(files) => {
                        for file in files {
                            network_files.push((file, peer.clone()));
                        }
                    }
                    Err(e) => println!("Erro ao listar arquivos do peer {}: {}", peer, e)
                }
            }
        }
        
        Ok(network_files)
    }

    pub async fn download_blocks_from_peers(&self, peers: Vec<String>, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks: Vec<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + 'static>>>> = Vec::new();
        
        // Cria um diretório de downloads se não existir
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("downloads"));
        tokio::fs::create_dir_all(&download_dir).await?;
        
        // Extrai apenas o nome do arquivo do caminho completo
        let file_name_only = std::path::Path::new(file_name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        // Cria o caminho completo para o arquivo de download
        let download_path = download_dir.join(&file_name_only);
        println!("Arquivo será salvo em: {}", download_path.display());
    
        for peer in peers {
            if peer != format!("{}:{}", self.ip, self.port) {
                let peer_clone = peer.clone();
                let file_name_clone = file_name.to_string();
                let peer_self = self.clone();
                let download_path = download_path.clone();
                let file_name_only_clone = file_name_only.clone(); // Clonando a String aqui

                tasks.push(tokio::spawn(async move {
                    println!("Tentando conectar ao peer: {}", peer_clone);
                    match TcpStream::connect(&peer_clone).await {
                        Ok(mut socket) => {
                            let request = format!("REQUEST_FILE {}", file_name_clone);
                            match socket.write_all(request.as_bytes()).await {
                                Ok(_) => {
                                    match peer_self.receive_file_in_blocks(download_path.to_str().unwrap(), &mut socket).await {
                                        Ok(_) => {
                                            println!("Download do arquivo {} concluído", file_name_only_clone);
                                            Ok(())
                                        },
                                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + 'static>)
                                    }
                                },
                                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + 'static>)
                            }
                        }
                        Err(e) => {
                            println!("Erro ao conectar ao peer {}: {}", peer_clone, e);
                            Err(Box::new(e) as Box<dyn std::error::Error + Send + 'static>)
                        }
                    }
                }));
            }
        }
    
        for task in tasks {
            match task.await {
                Ok(result) => {
                    if let Err(e) = result {
                        println!("Erro durante o download: {}", e);
                    }
                }
                Err(e) => println!("Erro na task: {}", e),
            }
        }
    
        // Verifica se o arquivo existe após o download
        match tokio::fs::metadata(&download_path).await {
            Ok(metadata) => {
                println!("\nDownload concluído com sucesso!");
                println!("Arquivo salvo em: {}", download_path.display());
                println!("Tamanho do arquivo: {} bytes", metadata.len());
            }
            Err(_) => {
                println!("Erro: Arquivo não foi encontrado após o download!");
            }
        }
    
        Ok(())
    }

    pub async fn unify_blocks(&self, file_path: &str, blocks: Vec<String>) -> std::io::Result<()> {
        let mut file = File::create(file_path).await?;

        for block in blocks {
            let mut block_file = File::open(&block).await?;
            let mut buffer = vec![0; 1024 * 1024];
            while let Ok(n) = block_file.read(&mut buffer).await {
                if n == 0 { break; }
                file.write_all(&buffer[..n]).await?;
            }
        }

        Ok(())
    }

    pub async fn validate_final_file(&self, file_path: &str, expected_checksum: &str) -> std::io::Result<()> {
        let mut file = File::open(file_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        let calculated_checksum = Self::calculate_checksum(&buffer);

        if calculated_checksum == expected_checksum {
            println!("Arquivo completo válido");
        } else {
            println!("Arquivo completo inválido");
        }

        Ok(())
    }

    pub async fn register_with_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("REGISTER {}:{}:{}", self.name, self.ip, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Registrado no tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn unregister_from_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("UNREGISTER {}:{}", self.name, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Desregistrado do tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn get_peers_from_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        stream.write_all(b"GET_PEERS").await?;

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let peer_list = String::from_utf8_lossy(&buffer[..n]).to_string();
        let peers = peer_list.split(',').map(|s| s.to_string()).collect();
        Ok(peers)
    }

    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).await?;
        println!("Peer rodando em {}:{}", self.ip, self.port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let shared_files = self.shared_files.clone();

            let peer_self = self.clone();
            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                if let Ok(n) = socket.read(&mut buffer).await {
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();

                    if request.starts_with("LIST_FILES") {
                        let file_list = shared_files.join(",");
                        socket.write_all(file_list.as_bytes()).await.unwrap();
                    }
                    
                    if request.starts_with("LIST_FILES") {
                        println!("Recebida solicitação de listagem de arquivos");
                        let file_list = shared_files.join(",");
                        println!("Enviando lista de arquivos: {}", file_list);
                        socket.write_all(file_list.as_bytes()).await.unwrap();
                    }

                    if !request.is_empty() {
                        println!("Mensagem recebida: {}", request);
                    }
                }
            });
        }
    }
}

pub fn list_local_files(directory: Option<&str>) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    
    let directories = match directory {
        Some(dir) => vec![PathBuf::from(dir)],
        None => vec![
            dirs::home_dir().unwrap_or_default().join("Documents"),
            dirs::home_dir().unwrap_or_default().join("Downloads"),
        ]
    };

    for dir in directories {
        if let Ok(entries) = read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            files.push((file_name, entry.path()));
                        }
                    }
                }
            }
        }
    }
    
    files
} 