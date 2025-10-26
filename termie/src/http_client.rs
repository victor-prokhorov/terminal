use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

pub async fn locally_classify(
    input: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let url: hyper::Uri = "http://localhost:11434/api/generate".parse()?;
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(11434);
    let addr = format!("{host}:{port}");
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("connection failed: {err:?}");
        }
    });
    let prompt = format!(
        "Classify input as either 'command' (UNIX shell) or 'natural' (normal natural human language). Respond only with 'command' or 'natural'. Examples:\n\nInput: \"ls -la\"\nOutput: command\n\nInput: \"echo hello\"\nOutput: command\n\nInput: \"Hello, how are you?\"\nOutput: natural\n\nNow classify this input:\nInput: \"{}\"",
        input.trim()
    );
    let body = serde_json::json!({
        "model": "qwen2.5:0.5b",
        "prompt": prompt,
        "stream": false
    });
    let body_bytes = serde_json::to_vec(&body)?;
    let req = Request::builder()
        .method("POST")
        .uri("/api/generate")
        .header("Host", host)
        .header("Content-Type", "application/json")
        .body(Full::<Bytes>::new(Bytes::from(body_bytes)))?;
    let res = sender.send_request(req).await?;
    let body_bytes = res.collect().await?.to_bytes();
    let response: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let response = response["response"]
        .as_str()
        .unwrap_or("natural")
        .to_lowercase();
    let is_command = response.contains("command");
    println!("took {:#?} to classify", start.elapsed());
    Ok(is_command)
}

pub async fn send_to_remote_llm(
    input: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let url: hyper::Uri = "http://localhost:11434/api/generate".parse()?;
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(11434);
    let addr = format!("{host}:{port}");
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("connection failed: {err:?}");
        }
    });
    let body = serde_json::json!({
        "model": "qwen2.5:0.5b",
        "prompt": input.trim(),
        "stream": false
    });
    let body_bytes = serde_json::to_vec(&body)?;
    let req = Request::builder()
        .method("POST")
        .uri("/api/generate")
        .header("Host", host)
        .header("Content-Type", "application/json")
        .body(Full::<Bytes>::new(Bytes::from(body_bytes)))?;
    let res = sender.send_request(req).await?;
    let body_bytes = res.collect().await?.to_bytes();
    let response: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let response_text = response["response"].as_str().unwrap_or("").to_string();
    println!("took {:#?} to process with remote LLM", start.elapsed());
    Ok(response_text)
}
