mod peer;
mod tracker;
mod chat;

use crate::peer::{Peer, list_local_files};
use crate::tracker::Tracker;
use crate::chat::{ChatServer, start_chat_client, message_receiver};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::env;
use std::io::{self, Write};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: cargo run -- tracker | cargo run -- peer");
        return;
    }
    let mode = &args[1];

    if mode == "tracker" {
        let tracker = Arc::new(Mutex::new(Tracker::new()));
        println!("Iniciando o tracker...");
        tracker.lock().await.start(6881).await.unwrap();
    } else if mode == "peer" {
        print!("Digite seu nome de peer: ");
        io::stdout().flush().unwrap();
        let mut peer_name = String::new();
        io::stdin().read_line(&mut peer_name).unwrap();
        let peer_name = peer_name.trim().to_string();
        
        let peer_port: u16 = 6882 + rand::random::<u16>() % 1000;
        let peer = Arc::new(Peer::new(
            "127.0.0.1".to_string(),
            peer_port,
            list_local_files("shared_files"),
            peer_name.clone(),
        ));

        // Registrar o peer no tracker
        peer.register_with_tracker("127.0.0.1", 6881).await.unwrap();

        let peer_clone = Arc::clone(&peer);
        tokio::spawn(async move {
            peer_clone.start_server().await.unwrap();
        });

        // Criação do canal para comunicação das mensagens
        let (sender, receiver) = mpsc::channel(100);
        let chat_server = ChatServer::new(sender);

        tokio::spawn(async move {
            chat_server.start_chat_server(peer_port + 1000).await.unwrap();
        });

        // Escuta de mensagens em paralelo
        tokio::spawn(async move {
            message_receiver(receiver).await;
        });

        // Comandos no terminal
        println!("Digite 'list' para obter a lista de peers, 'chat' para iniciar um chat ou 'exit' para sair.");
        loop {
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            let command = command.trim().to_string();

            if command == "list" {
                let peers = peer.get_peers_from_tracker("127.0.0.1", 6881).await.unwrap();
                println!("Lista de Peers: {:?}", peers);
            } else if command == "chat" {
                print!("Digite o número da porta do peer para iniciar o chat: ");
                io::stdout().flush().unwrap();
                let mut target_port_str = String::new();
                io::stdin().read_line(&mut target_port_str).unwrap();
                let target_port: u16 = target_port_str.trim().parse().unwrap();
                
                // Inicia o chat com o peer desejado
                start_chat_client(target_port + 1000).await.unwrap();
            } else if command == "exit" {
                // Desregistrar do tracker
                peer.unregister_from_tracker("127.0.0.1", 6881).await.unwrap();
                println!("Desconectando do tracker...");
                break;
            }
        }
    }
}
