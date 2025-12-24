mod chat;

use chat::{ChatManager, Conversation, Message};
use std::sync::Mutex;
use tauri::State;

struct AppState {
    chat_manager: Mutex<ChatManager>,
}

#[tauri::command]
fn get_conversations(state: State<AppState>) -> Vec<Conversation> {
    let manager = state.chat_manager.lock().unwrap();
    manager.get_conversations()
}

#[tauri::command]
fn get_conversation(state: State<AppState>, id: String) -> Option<Conversation> {
    let manager = state.chat_manager.lock().unwrap();
    manager.get_conversation(&id)
}

#[tauri::command]
fn create_conversation(state: State<AppState>, title: Option<String>) -> Conversation {
    let mut manager = state.chat_manager.lock().unwrap();
    manager.create_conversation(title)
}

#[tauri::command]
fn send_message(state: State<AppState>, conversation_id: String, content: String) -> Vec<Message> {
    let mut manager = state.chat_manager.lock().unwrap();
    manager.send_message(&conversation_id, content)
}

#[tauri::command]
fn delete_conversation(state: State<AppState>, id: String) -> bool {
    let mut manager = state.chat_manager.lock().unwrap();
    manager.delete_conversation(&id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            chat_manager: Mutex::new(ChatManager::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_conversations,
            get_conversation,
            create_conversation,
            send_message,
            delete_conversation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
