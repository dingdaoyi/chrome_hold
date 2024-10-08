use std::env;
use std::error::Error;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use axum::Router;
use axum::routing::get;
use std::path:: PathBuf;
use axum::http::Method;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::{Instant, sleep};
use tracing::{error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tower_http::cors::{Any, CorsLayer};

static AUTH_CONFIG: OnceLock<Sender<Instant>> = OnceLock::new();

pub async fn start() -> Result<(), Box<dyn Error>> {
    setup_logger().await?;
    println!("本程序是判断浏览器卡死的程序,机器人自动运行期间请不要关闭!");
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
    let cors = CorsLayer::new()
        .allow_origin(Any) // 或者设置为特定的 origin
        .allow_methods(vec![Method::GET,Method::POST,Method::OPTIONS]);
    axum::serve(listener, Router::new()
        .route("/ping", get(ping))
        .layer(cors)).await?;

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
    let mut log_path = get_exe_path();
    log_path.push("logs");
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_path, "prefix.log");

    if cfg!(debug_assertions) {
        // 在 debug 模式下，只打印到控制台
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)  // 设置日志级别
            .init();
    } else {
        // 在 release 模式下，打印到文件
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_max_level(tracing::Level::INFO)  // 设置日志级别
            .init();
    }

    Ok(())
}
fn get_exe_path() -> PathBuf {
    let current_dir = env::current_exe().expect("Failed to get current exe path");
    let  log_path = PathBuf::from(current_dir.parent().expect("Failed to get parent directory"));
    log_path
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
            Err(_msg) => {}
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("taskkill")
            .args(&["/F", "/IM", "chrome.exe"])
            .output()
            .expect("没有删除掉");

        if output.status.success() {
            info!("关闭成功");
        } else {
            info!("关闭失败");
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("pkill")
            .arg("-TERM") // 使用 SIGTERM
            .arg("chrome")
            .output()
            .expect("无法关闭 Chrome");

        if output.status.success() {
            info!("Chrome 已正常关闭");
        } else {
            info!("Chrome 关闭失败，退出代码: {}", output.status.code().unwrap_or(-1));
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
        let mut script_path = get_exe_path();
        script_path.push("start_bat.bat");
        info!("脚本路径为:{:?}", &script_path);

        let output = Command::new("cmd")
            .args(&["/C", script_path.to_str().expect("Invalid script path")])
            .output();

        match output {
            Ok(output) => {
                // 将 stdout 和 stderr 从 GBK 编码转换为 UTF-8
                let (stdout, _, _) = encoding_rs::GBK.decode(&output.stdout);
                let (stderr, _, _) = encoding_rs::GBK.decode(&output.stderr);

                info!("stdout: {:?}", stdout);
                info!("stderr: {:?}", stderr);

                if output.status.success() {
                    info!("Chrome 启动成功");
                } else {
                    error!("Chrome 启动失败，退出代码: {:?}", output.status.code());
                }
            }
            Err(err) => {
                error!("启动 Chrome 失败: {:?}", err);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let child = Command::new("google-chrome")
            .spawn();
        match child {
            Ok(child) => {
                if child.id() > 0 {
                    info!("Chrome 启动成功，进程 ID: {}", child.id());
                } else {
                    error!("Chrome 启动失败:{:?}",child.stdout);
                }
            }
            Err(msg) => {
                info!("Chrome 启动失败:{:?}",msg);
            }
        }
    }
}