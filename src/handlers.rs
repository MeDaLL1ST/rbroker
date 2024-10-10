use super::model::{ALL_CONS, ALL_USES, INFO_USES, STORE};
use axum::body::Bytes;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use prometrics_sb::axpromlib::{auth, get_str_var, Atp};
use serde_json::Value;
use tokio::sync::mpsc::Receiver;

///Handler for sending a value to the STORE
pub async fn add_handler(State(pool): State<Atp>, req: Request) -> Result<Json<Value>, StatusCode> {
    let hdrs = req.headers().get("Authorization").cloned();
    let body_bytes: Bytes = axum::body::to_bytes(req.into_body(), 256)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|_| StatusCode::BAD_REQUEST)?;

    let parsed_json: serde_json::Value =
        serde_json::from_str(&body_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    let key = parsed_json
        .get("key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let value = parsed_json
        .get("value")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_REQUEST)?;

    pool.handle_request(move || {
        if hdrs.unwrap().to_str().unwrap() != get_str_var("API_KEY") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        ALL_USES.inc();

        STORE.rpush(key.clone(), value.clone());
        let parsed_json = serde_json::json!({
            "key": key,
            "value": value
        });

        Ok(Json(parsed_json))
    })
    .await?
}

///Handler for output of all allocated keys
pub async fn list_handler(
    State(pool): State<Atp>,
    req: Request,
) -> Result<Json<Value>, StatusCode> {
    let list = STORE.list().await;
    pool.handle_request(move || {
        INFO_USES.inc();
        match auth(req) {
            Ok(true) => {}
            Err(_err) => return Err(StatusCode::UNAUTHORIZED),
            _ => {}
        }

        let data = serde_json::json!({
            "keys": list
        });

        //{"keys": [list]}
        Ok(Json(data))
    })
    .await?
}

///Handler for displaying the number of subscriptions per key
pub async fn info_handler(
    State(pool): State<Atp>,
    req: Request,
) -> Result<Json<Value>, StatusCode> {
    let hdrs = req.headers().get("Authorization").cloned();
    let body_bytes: Bytes = axum::body::to_bytes(req.into_body(), 256)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|_| StatusCode::BAD_REQUEST)?;

    let parsed_json: serde_json::Value =
        serde_json::from_str(&body_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    let key = parsed_json
        .get("key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let keya = STORE.info(key).await;

    pool.handle_request(move || {
        INFO_USES.inc();
        if hdrs.unwrap().to_str().unwrap() != get_str_var("API_KEY") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let data = serde_json::json!({
            "uses": keya
        });

        Ok(Json(data))
    })
    .await?
}

pub async fn subs_handler(ws: WebSocketUpgrade, req: Request) -> impl axum::response::IntoResponse {
    match auth(req) {
        Ok(true) => ws.on_upgrade(handle_socket),
        _ => axum::http::StatusCode::UNAUTHORIZED.into_response(),
    }
}

///Websocket connection handler
pub async fn handle_socket(mut socket: WebSocket) {
    ALL_CONS.inc();
    let mut current_rc: Option<Receiver<String>> = None;
    let mut current_key: Option<String> = None;
    let mut current_id: Option<usize> = None;

    loop {
        tokio::select! {
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if let Ok(text) = msg.to_text() {
                            if !text.is_empty() {
                                let parsed_json: serde_json::Value =
                                    serde_json::from_str(text).expect("");

                                let key = parsed_json
                                    .get("key")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .ok_or(StatusCode::BAD_REQUEST)
                                    .unwrap();

                                if let Some(prev_key) = &current_key {
                                    if let Some(prev_id) = current_id {
                                        STORE.dec_key(prev_key.to_string(), prev_id).await;
                                    }
                                }

                                current_key = Some(key.clone());

                                let (rc, id) = STORE.get_updates(key.clone()).await;

                                current_rc = Some(rc);
                                current_id = Some(id);
                            } else {
                                if let Some(key) = &current_key {
                                    if let Some(id) = current_id {
                                        STORE.dec_key(key.to_string(), id).await;
                                    }
                                }
                                ALL_CONS.dec();
                                return;
                            }
                        } else {
                            if let Some(key) = &current_key {
                                if let Some(id) = current_id {
                                    STORE.dec_key(key.to_string(), id).await;
                                }
                            }
                            ALL_CONS.dec();
                            return;
                        }
                    }
                    Some(Err(_)) | None => {
                        if let Some(key) = &current_key {
                            if let Some(id) = current_id {
                                STORE.dec_key(key.to_string(), id).await;
                            }
                        }
                        ALL_CONS.dec();
                        return;
                    }
                }
            }

            update = async {
                if let Some(receiver) = &mut current_rc {
                    receiver.recv().await
                } else {
                    None
                }
            }, if current_rc.is_some() => {
                if let Some(message) = update {
                    let _ = socket
                        .send(axum::extract::ws::Message::Text(message.to_string()))
                        .await;
                } else {
                    current_rc = None;
                }
            }
        }
    }
}
