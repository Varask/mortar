use std::io::{self, BufRead};

use mortar::server::build_app_with_state;
use mortar::server_cli::{handle_cli_command, print_prompt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Determine data path
    let data_path = if std::path::Path::new("data").exists() {
        "data"
    } else if std::path::Path::new("/workspace/rust/mortar/data").exists() {
        "/workspace/rust/mortar/data"
    } else {
        "data"
    };

    // Determine web assets path
    let web_path = if std::path::Path::new("src/web").exists() {
        "src/web"
    } else if std::path::Path::new("/workspace/rust/mortar/src/web").exists() {
        "/workspace/rust/mortar/src/web"
    } else {
        "src/web"
    };

    // Build router + shared state from library
    let (app, state) = build_app_with_state(data_path, web_path);

    let addr = "0.0.0.0:3000";
    println!("Server starting on http://{addr}");
    println!("Web assets from: {web_path}");
    println!("Ballistics from: {data_path}");
    println!();

    let interactive = atty::is(atty::Stream::Stdin);

    if interactive {
        // Spawn web server in background
        let listener = TcpListener::bind(addr).await.unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // CLI loop
        let stdin = io::stdin();
        let reader = stdin.lock();

        print_prompt();

        for line in reader.lines() {
            match line {
                Ok(input) => {
                    if input.trim() == "exit" || input.trim() == "quit" || input.trim() == "q" {
                        println!("Shutting down...");
                        break;
                    }
                    handle_cli_command(&input, &state).await;
                }
                Err(_) => break,
            }

            print_prompt();
        }
    } else {
        println!("Running in non-interactive mode (web server only)");
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}
