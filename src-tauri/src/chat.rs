use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ChatManager {
    conversations: Vec<Conversation>,
}

impl ChatManager {
    pub fn new() -> Self {
        let mut manager = Self {
            conversations: Vec::new(),
        };

        // Create some mock conversations for demo
        manager.create_mock_data();
        manager
    }

    fn create_mock_data(&mut self) {
        // First conversation with history
        let conv1_id = Uuid::new_v4().to_string();
        let conv1 = Conversation {
            id: conv1_id.clone(),
            title: "Rust async patterns".to_string(),
            messages: vec![
                Message {
                    id: Uuid::new_v4().to_string(),
                    role: "user".to_string(),
                    content: "Can you explain async/await in Rust?".to_string(),
                    timestamp: Utc::now(),
                },
                Message {
                    id: Uuid::new_v4().to_string(),
                    role: "assistant".to_string(),
                    content: "Async/await in Rust allows you to write asynchronous code that looks synchronous. The `async` keyword transforms a block of code into a state machine that implements the `Future` trait. When you call `.await` on a future, it yields control back to the executor if the operation isn't ready yet.\n\nHere's a simple example:\n\n```rust\nasync fn fetch_data() -> Result<String, Error> {\n    let response = reqwest::get(\"https://api.example.com\").await?;\n    let body = response.text().await?;\n    Ok(body)\n}\n```\n\nThe key thing to understand is that futures in Rust are lazy - they don't do anything until polled.".to_string(),
                    timestamp: Utc::now(),
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.conversations.push(conv1);

        // Second conversation
        let conv2_id = Uuid::new_v4().to_string();
        let conv2 = Conversation {
            id: conv2_id.clone(),
            title: "Building a web server".to_string(),
            messages: vec![
                Message {
                    id: Uuid::new_v4().to_string(),
                    role: "user".to_string(),
                    content: "What's the best way to build a REST API in Rust?".to_string(),
                    timestamp: Utc::now(),
                },
                Message {
                    id: Uuid::new_v4().to_string(),
                    role: "assistant".to_string(),
                    content: "For building REST APIs in Rust, you have several excellent options:\n\n1. **Axum** - Built by the Tokio team, modern and ergonomic\n2. **Actix-web** - Very fast, actor-based architecture\n3. **Rocket** - Great developer experience with macros\n4. **Warp** - Composable, filter-based approach\n\nI'd recommend starting with Axum for new projects - it has great documentation and integrates well with the Tokio ecosystem.".to_string(),
                    timestamp: Utc::now(),
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.conversations.push(conv2);

        // Third conversation
        let conv3_id = Uuid::new_v4().to_string();
        let conv3 = Conversation {
            id: conv3_id.clone(),
            title: "Error handling strategies".to_string(),
            messages: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.conversations.push(conv3);
    }

    pub fn get_conversations(&self) -> Vec<Conversation> {
        self.conversations.clone()
    }

    pub fn get_conversation(&self, id: &str) -> Option<Conversation> {
        self.conversations.iter().find(|c| c.id == id).cloned()
    }

    pub fn create_conversation(&mut self, title: Option<String>) -> Conversation {
        let conv = Conversation {
            id: Uuid::new_v4().to_string(),
            title: title.unwrap_or_else(|| "New conversation".to_string()),
            messages: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.conversations.push(conv.clone());
        conv
    }

    pub fn send_message(&mut self, conversation_id: &str, content: String) -> Vec<Message> {
        if let Some(conv) = self
            .conversations
            .iter_mut()
            .find(|c| c.id == conversation_id)
        {
            // Add user message
            let user_msg = Message {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                content: content.clone(),
                timestamp: Utc::now(),
            };
            conv.messages.push(user_msg);

            // Generate mock AI response
            let ai_response = self.generate_mock_response(&content);
            let ai_msg = Message {
                id: Uuid::new_v4().to_string(),
                role: "assistant".to_string(),
                content: ai_response,
                timestamp: Utc::now(),
            };
            conv.messages.push(ai_msg);

            // Update title if first message
            if conv.messages.len() == 2 {
                conv.title = content.chars().take(30).collect::<String>() + "...";
            }

            conv.updated_at = Utc::now();
            conv.messages.clone()
        } else {
            Vec::new()
        }
    }

    fn generate_mock_response(&self, user_message: &str) -> String {
        // Simple mock responses based on keywords
        let lower = user_message.to_lowercase();

        if lower.contains("hello") || lower.contains("hi") {
            "Hello! I'm Kasate, your AI assistant. How can I help you today?".to_string()
        } else if lower.contains("rust") {
            "Rust is a fantastic systems programming language focused on safety, concurrency, and performance. It's perfect for building reliable software. What specific aspect of Rust would you like to explore?".to_string()
        } else if lower.contains("help") {
            "I'm here to help! I can assist you with:\n\n- Programming questions\n- Code explanations\n- Problem-solving\n- Learning new concepts\n\nWhat would you like to know?".to_string()
        } else if lower.contains("tauri") {
            "Tauri is an excellent framework for building cross-platform desktop and mobile applications using Rust for the backend and web technologies for the frontend. It's lightweight and secure!".to_string()
        } else {
            format!("That's an interesting question about \"{}\". In a production app, I would connect to an AI service to provide a meaningful response. For now, this is a mock response demonstrating the chat interface.",
                user_message.chars().take(50).collect::<String>())
        }
    }

    pub fn delete_conversation(&mut self, id: &str) -> bool {
        let initial_len = self.conversations.len();
        self.conversations.retain(|c| c.id != id);
        self.conversations.len() != initial_len
    }
}
