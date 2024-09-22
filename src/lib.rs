use std::env;
use std::error::Error;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use axum::extract::State;
use axum::Router;
use axum::routing::get;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::{Instant, sleep};
use tracing::info;
static AUTH_CONFIG: OnceLock<Sender<Instant>> = OnceLock::new();
pub async fn start() -> Result<(), Box<dyn Error>> {
    setup_logger().await?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:7724").await.unwrap();
    info!("服务启动成功:127.0.0.1:7724");

    let (tx, mut rx) = mpsc::channel::<Instant>(32);

    AUTH_CONFIG.set(tx).expect("初始化错误");
    // 记录最后一次收到 Ping 的时间
    let timeout_duration = Duration::from_secs(20);
    let mut last_ping: Option<Instant> = None;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(timeout_duration) => {
                    if let Some(last) = last_ping {
                        if last.elapsed() >= timeout_duration {
                            info!("超时未收到 Ping，执行操作");
                            //操作重启浏览器,并重置last_ping.
                            restart_chrome().await;
                            last_ping=None;
                        }
                    }
                }
                data = rx.recv() => {
                    if let Some(data) = data {
                        info!("收到 Ping，重置定时器");
                        last_ping = Some(data);
                    }
                }
            }
        }
    });
    axum::serve(listener, Router::new()
        .route("/ping", get(ping))).await?;

    Ok(())
}

async fn restart_chrome() {
    close_chrome();
    sleep(Duration::from_secs(5)).await;
    start_chrome();
}



async fn ping() -> String {
    let last_ping = Instant::now();
    AUTH_CONFIG.get().unwrap()
        .send(last_ping).await.unwrap();
    info!("发送 Ping 信号");
    return "pong".to_string();
}

async fn setup_logger() -> Result<(), Box<dyn Error>> {
    env::set_var("RUST_LOG", "info");
    tracing_subscriber::fmt::init();
    Ok(())
}

fn close_chrome() {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("pkill")
            .arg("Google Chrome")
            .output();
        match output {
            Ok(output) => {
                if output.status.success() {
                    info!("关闭成功");
                } else {
                    info!("关闭失败");
                }
            }
            Err(msg) => {}
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("taskkill")
            .args(&["/F", "/IM", "chrome.exe"])
            .output()
            .expect("没有删除掉");

        if output.status.success() {
            println!("关闭成功");
        } else {
            println!("关闭失败");
        }
    }
}



fn start_chrome() {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("open")
            .arg("-a")
            .arg("Google Chrome")
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    info!("Chrome 启动成功");
                } else {
                    info!("Chrome 启动失败");
                }
            }
            Err(err) => {
                info!("启动 Chrome 失败: {:?}", err);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let current_dir = env::current_exe().expect("Failed to get current exe path");
        let mut script_path = PathBuf::from(current_dir.parent().expect("Failed to get parent directory"));
        script_path.push("start_bat.bat");
        let output = Command::new("cmd")
            .args(&["/C", script_path.to_str().expect("Invalid script path")])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    info!("Chrome 启动成功");
                } else {
                    info!("Chrome 启动失败");
                }
            }
            Err(err) => {
                info!("启动 Chrome 失败: {:?}", err);
            }
        }
    }
}