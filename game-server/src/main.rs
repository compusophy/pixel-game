use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use game_core::protocol::{ClientMsg, ServerMsg};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tower_http::services::ServeDir;

mod game_world;
use game_world::GameWorld;

type SharedWorld = Arc<Mutex<GameWorld>>;
type Tx = mpsc::UnboundedSender<ServerMsg>;

struct AppState {
    world: SharedWorld,
    txs: Arc<Mutex<std::collections::HashMap<u32, Tx>>>,
}

#[tokio::main]
async fn main() {
    let world = Arc::new(Mutex::new(GameWorld::new()));
    let txs = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let state = Arc::new(AppState {
        world: world.clone(),
        txs: txs.clone(),
    });

    let app = Router::new()
        .fallback_service(ServeDir::new("static"))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // spawn game loop
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        loop {
            interval.tick().await;
            let mut w = world.lock().await;
            let msg = w.tick(50.0);
            drop(w); // drop lock before broadcasting

            // broadcast tick to all clients
            let mut clients = txs.lock().await;
            clients.retain(|_, tx| tx.send(msg.clone()).is_ok());
        }
    });

    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMsg>();

    let mut player_id = 0;

    // Wait for the Join message to spawn the player
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Binary(bin) = msg {
            if let Ok(client_msg) = bincode::deserialize::<ClientMsg>(&bin) {
                if let ClientMsg::Join { name } = client_msg {
                    let mut world = state.world.lock().await;
                    player_id = world.add_player(name);
                    let welcome = ServerMsg::Welcome {
                        player_id,
                        map_seed: world.map_seed,
                    };
                    let _ = tx.send(welcome);
                    state.txs.lock().await.insert(player_id, tx.clone());
                    break;
                }
            }
        }
    }

    if player_id == 0 {
        return; // Left before joining
    }

    // Task to forward ServerMsg to the websocket (bincode-encoded)
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(bin) = bincode::serialize(&msg) {
                if sender.send(Message::Binary(bin.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task to read from websocket and forward to GameWorld as ClientMsg
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Binary(bin))) = receiver.next().await {
            if let Ok(client_msg) = bincode::deserialize::<ClientMsg>(&bin) {
                state_clone.world.lock().await.handle_client_msg(player_id, client_msg);
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    // Cleanup when done
    state.txs.lock().await.remove(&player_id);
    state.world.lock().await.remove_player(player_id);
}
