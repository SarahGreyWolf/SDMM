
use interprocess::local_socket::LocalSocketListener;
use serde::{Deserialize, Serialize};
use core::panic;
use std::fs::{File};
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::TrySendError::Disconnected;
use std::sync::mpsc::{SyncSender};
use std::thread;
use futures_util::StreamExt;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Links(Vec<DownloadLink>);

#[derive(Deserialize, Serialize, Debug, Clone)]
struct DownloadLink {
    name: String,
    short_name: String,
    #[serde(rename = "URI")]
    uri: String,
}

pub fn handle_download_requests(sync_sender: SyncSender<(String, usize, usize)>, download_path: PathBuf) {
    let base_path = Path::new("api.nexusmods.com/v1/games/");
    let listener = LocalSocketListener::bind("/tmp/sdmm.sock").unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("download_handler")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Err(e) = &stream {
                eprintln!("Stream Error: {e}");
            }
            let mut stream = stream.unwrap();
            let download_path = download_path.clone();
            let sync_sender = sync_sender.clone();
            // Get sent URL string
            let mut buffer = vec![0u8; 1024];
            stream.read_exact(&mut buffer).unwrap();
            runtime.spawn(async move {
                let sync_sender = sync_sender.clone();
                let client = reqwest::Client::new();
                let mut string = String::from_utf8(buffer).unwrap();
                string = string.replace('\0', "");
                // Make a request to get the Download URL
                let response = client.get(get_download_url(base_path, &string))
                // This needs to be less static and with a proper API key from Nexus for
                // the application
                .header("apikey", "")
                .send().await;
                match response {
                    Ok(resp) => {
                        let response_body = resp.json::<Links>().await;
                        match response_body {
                            Ok(links) => {
                                let download = links.0.first().unwrap();
                                println!("Beginning Download from {:?}", download.uri);
                                download_file(&client, sync_sender, download, &download_path).await;
                                println!("Finished Download of {}", get_filename(&download.uri));
                            }
                            Err(e) => eprintln!("Error Occured: {}", e),
                        }
                        
                    }
                    Err(e) => {
                        eprintln!("Error Occured: {}", e);
                    }
                }
            });
        }
    });
}

async fn download_file(client: &reqwest::Client, sync_sender: SyncSender<(String, usize, usize)>, download: &DownloadLink, download_path: &Path) {
    let download_clone = download.clone();
    let download_path_clone = download_path.clone();
    let client = client.clone();
    let sync_sender = sync_sender.clone();
    let res = client.get(&download_clone.uri).send().await;
    if let Ok(resp) = res {
        let total_size =
            if let Some(size) = resp.content_length() {
                size as usize
            } else {
                eprintln!(
                    "Failed to download from {}",
                    &download_clone.uri
                );
                0
            };
        if total_size == 0 {
            eprintln!("File size is 0");
            return;
        }
        let file_name = get_filename(&download_clone.uri);
        if let Ok(mut file) =
            File::create(download_path_clone.join(&file_name))
        {
            let mut downloaded: usize = 0;
            match sync_sender.try_send((file_name.to_string(), downloaded, total_size)) {
                Err(e) if e == Disconnected((file_name.to_string(), downloaded, total_size)) => {
                    panic!("Error Occured when sending: {}", e);
                }
                _ => {}
            }
            let mut stream = resp.bytes_stream();
            while let Some(item) = stream.next().await {
                if let Ok(chunk) = item {
                    let written = file.write(&chunk).unwrap();
                    assert_eq!(written, chunk.len());
                    downloaded += chunk.len();
                    match sync_sender.try_send((file_name.to_string(), downloaded, total_size)) {
                        Err(e) if e == Disconnected((file_name.to_string(), downloaded, total_size)) => {
                            panic!("Error Occured when sending: {}", e);
                        }
                        _ => {}
                    }
                } else {
                    eprintln!("Failed to create file at {}", download_path_clone.join(file_name).display());
                    return;
                }
            }
            sync_sender.send((file_name.to_string(), downloaded, total_size)).unwrap();
        } else {
            eprintln!("Failed to create file at {}", download_path_clone.join(file_name).display());
        }
    } else {
        eprintln!(
            "Failed to download from {}",
            &download_clone.uri
        );
    }
}

fn get_filename(uri: &str) -> String {
    let split_uri = uri
        .split('/')
        .collect::<Vec<&str>>();
    let file_name = if split_uri[2].contains("nexus-cdn") {
        split_uri[5]
    } else {
        split_uri[6]
    };
    file_name.split('?').next().unwrap().to_string()
}

// Convert URL String in Path, excluding the nxm:// part
fn get_download_url(base_path: &Path, requested_uri: &str) -> String {
    let url = requested_uri.split_at(6).1;
    let url: Vec<&str> = url.split('?').collect();
    let path = Path::new(&url[0]);
    let queries = url[1];
    format!(
        "https://{}{}/download_link.json?{}",
        base_path.display(),
        path.display(),
        &queries
    )
}
