impl Tracker {
    // ... código existente ...

    pub async fn get_peers(&self) -> Result<Vec<Peer>, Box<dyn Error>> {
        let request = TrackerRequest::new(6881);
        
        let url = format!(
            "{}?peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}",
            self.announce_url,
            request.peer_id,
            request.port,
            request.uploaded,
            request.downloaded,
            request.left,
            request.compact
        );

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;

        if response.status().is_success() {
            let tracker_response: TrackerResponse = response.json().await?;
            println!("Número de peers: {}", tracker_response.peers.len());
            Ok(tracker_response.peers)
        } else {
            Err(Box::new(response.error_for_status().unwrap_err()))
        }
    }
}
