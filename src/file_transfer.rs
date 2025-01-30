// src/file_transfer.rs
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::fs::File;
use std::io::{BufReader, Read, Write};

pub async fn send_file(filename: &str, target_ip: &str, target_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = BufReader::new(File::open(filename)?);
    let mut buffer = [0; 1024];
    let mut stream = TcpStream::connect(format!("{}:{}", target_ip, target_port)).await?;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buffer[..n]).await?;
    }
    Ok(())
}

pub async fn receive_file(output_filename: &str, listen_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port)).await?;
    let (mut socket, _) = listener.accept().await?;
    let mut file = File::create(output_filename)?;
    let mut buffer = [0; 1024];
    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
    }
    Ok(())
}
