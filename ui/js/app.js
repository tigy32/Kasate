// Kasate - Cross-platform AI Chat Application

// Check if running in Tauri
const isTauri = window.__TAURI__ !== undefined;

// State
let currentConversationId = null;
let conversations = [];
let isListening = false;

// DOM Elements
const elements = {
    // Navigation
    navTabs: document.querySelectorAll('.nav-tab'),
    screens: document.querySelectorAll('.screen'),

    // Sidebar
    newChatBtn: document.getElementById('newChatBtn'),
    conversationList: document.getElementById('conversationList'),

    // Chat
    chatTitle: document.getElementById('chatTitle'),
    messagesContainer: document.getElementById('messagesContainer'),
    welcomeMessage: document.getElementById('welcomeMessage'),
    messageInput: document.getElementById('messageInput'),
    sendBtn: document.getElementById('sendBtn'),
    suggestionBtns: document.querySelectorAll('.suggestion-btn'),

    // Voice
    halEye: document.getElementById('halEye'),
    voiceStatus: document.getElementById('voiceStatus'),
    voiceBtn: document.getElementById('voiceBtn'),
    transcriptText: document.getElementById('transcriptText'),

    // History
    historyList: document.getElementById('historyList'),
    historySearch: document.getElementById('historySearch'),
};

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
    setupNavigation();
    setupChatInput();
    setupVoice();
    setupSuggestions();

    if (isTauri) {
        await loadConversations();
    } else {
        // Mock data for web preview
        loadMockData();
    }

    renderConversationList();
    renderHistoryList();
});

// Navigation
function setupNavigation() {
    elements.navTabs.forEach(tab => {
        tab.addEventListener('click', () => {
            const screen = tab.dataset.screen;
            switchScreen(screen);
        });
    });
}

function switchScreen(screenName) {
    // Update tabs
    elements.navTabs.forEach(tab => {
        tab.classList.toggle('active', tab.dataset.screen === screenName);
    });

    // Update screens
    elements.screens.forEach(screen => {
        screen.classList.toggle('active', screen.id === `${screenName}Screen`);
    });

    // Refresh history when switching to it
    if (screenName === 'history') {
        renderHistoryList();
    }
}

// Conversations
async function loadConversations() {
    if (isTauri) {
        conversations = await window.__TAURI__.core.invoke('get_conversations');
    }
}

function loadMockData() {
    conversations = [
        {
            id: '1',
            title: 'Rust async patterns',
            messages: [
                { id: '1a', role: 'user', content: 'Can you explain async/await in Rust?', timestamp: new Date().toISOString() },
                { id: '1b', role: 'assistant', content: 'Async/await in Rust allows you to write asynchronous code that looks synchronous. The `async` keyword transforms a block of code into a state machine that implements the `Future` trait.', timestamp: new Date().toISOString() },
            ],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
        },
        {
            id: '2',
            title: 'Building a web server',
            messages: [
                { id: '2a', role: 'user', content: "What's the best way to build a REST API in Rust?", timestamp: new Date().toISOString() },
                { id: '2b', role: 'assistant', content: 'For building REST APIs in Rust, you have several excellent options: Axum, Actix-web, Rocket, and Warp.', timestamp: new Date().toISOString() },
            ],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
        },
        {
            id: '3',
            title: 'Error handling strategies',
            messages: [],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
        },
    ];
}

function renderConversationList() {
    elements.conversationList.innerHTML = conversations.map(conv => `
        <div class="conversation-item ${conv.id === currentConversationId ? 'active' : ''}"
             data-id="${conv.id}">
            <div class="icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2">
                    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
                </svg>
            </div>
            <div class="details">
                <div class="title">${escapeHtml(conv.title)}</div>
                <div class="preview">${conv.messages.length} messages</div>
            </div>
            <button class="delete-btn" data-id="${conv.id}">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="3 6 5 6 21 6"></polyline>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
            </button>
        </div>
    `).join('');

    // Add click handlers
    document.querySelectorAll('.conversation-item').forEach(item => {
        item.addEventListener('click', (e) => {
            if (!e.target.closest('.delete-btn')) {
                selectConversation(item.dataset.id);
            }
        });
    });

    document.querySelectorAll('.conversation-item .delete-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            deleteConversation(btn.dataset.id);
        });
    });

    // New chat button
    elements.newChatBtn.onclick = createNewConversation;
}

async function selectConversation(id) {
    currentConversationId = id;
    const conv = conversations.find(c => c.id === id);

    if (conv) {
        elements.chatTitle.textContent = conv.title;
        renderMessages(conv.messages);
        renderConversationList();
        switchScreen('chat');
    }
}

async function createNewConversation() {
    let newConv;

    if (isTauri) {
        newConv = await window.__TAURI__.core.invoke('create_conversation', { title: null });
        conversations.unshift(newConv);
    } else {
        newConv = {
            id: Date.now().toString(),
            title: 'New conversation',
            messages: [],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
        };
        conversations.unshift(newConv);
    }

    currentConversationId = newConv.id;
    elements.chatTitle.textContent = newConv.title;
    elements.welcomeMessage.style.display = 'flex';
    elements.messagesContainer.querySelectorAll('.message').forEach(m => m.remove());
    renderConversationList();
    switchScreen('chat');
    elements.messageInput.focus();
}

async function deleteConversation(id) {
    if (isTauri) {
        await window.__TAURI__.core.invoke('delete_conversation', { id });
    }

    conversations = conversations.filter(c => c.id !== id);

    if (currentConversationId === id) {
        currentConversationId = null;
        elements.chatTitle.textContent = 'New Conversation';
        elements.welcomeMessage.style.display = 'flex';
        elements.messagesContainer.querySelectorAll('.message').forEach(m => m.remove());
    }

    renderConversationList();
    renderHistoryList();
}

// Chat
function setupChatInput() {
    elements.messageInput.addEventListener('input', () => {
        elements.sendBtn.disabled = !elements.messageInput.value.trim();
        autoResizeTextarea();
    });

    elements.messageInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            if (elements.messageInput.value.trim()) {
                sendMessage();
            }
        }
    });

    elements.sendBtn.addEventListener('click', sendMessage);
}

function autoResizeTextarea() {
    const textarea = elements.messageInput;
    textarea.style.height = 'auto';
    textarea.style.height = Math.min(textarea.scrollHeight, 200) + 'px';
}

function setupSuggestions() {
    elements.suggestionBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            elements.messageInput.value = btn.textContent;
            elements.sendBtn.disabled = false;
            sendMessage();
        });
    });
}

async function sendMessage() {
    const content = elements.messageInput.value.trim();
    if (!content) return;

    // Create conversation if needed
    if (!currentConversationId) {
        await createNewConversation();
    }

    // Hide welcome message
    elements.welcomeMessage.style.display = 'none';

    // Add user message to UI
    appendMessage({ role: 'user', content });

    // Clear input
    elements.messageInput.value = '';
    elements.sendBtn.disabled = true;
    autoResizeTextarea();

    // Show typing indicator
    const typingIndicator = document.createElement('div');
    typingIndicator.className = 'message assistant';
    typingIndicator.innerHTML = `
        <div class="message-avatar">K</div>
        <div class="message-content">
            <div class="message-role">Kasate</div>
            <div class="typing-indicator">
                <span></span>
                <span></span>
                <span></span>
            </div>
        </div>
    `;
    elements.messagesContainer.appendChild(typingIndicator);
    scrollToBottom();

    // Send to backend
    let messages;
    if (isTauri) {
        messages = await window.__TAURI__.core.invoke('send_message', {
            conversationId: currentConversationId,
            content,
        });
    } else {
        // Mock response for web
        await new Promise(r => setTimeout(r, 1000));
        messages = mockResponse(content);
    }

    // Remove typing indicator
    typingIndicator.remove();

    // Add assistant response
    const assistantMessage = messages[messages.length - 1];
    if (assistantMessage) {
        appendMessage(assistantMessage);
    }

    // Update conversation in list
    const conv = conversations.find(c => c.id === currentConversationId);
    if (conv) {
        conv.messages = messages;
        if (messages.length === 2) {
            conv.title = content.slice(0, 30) + '...';
            elements.chatTitle.textContent = conv.title;
        }
        renderConversationList();
    }
}

function mockResponse(userMessage) {
    const lower = userMessage.toLowerCase();
    let response;

    if (lower.includes('hello') || lower.includes('hi')) {
        response = "Hello! I'm Kasate, your AI assistant. How can I help you today?";
    } else if (lower.includes('rust')) {
        response = "Rust is a fantastic systems programming language focused on safety, concurrency, and performance. What specific aspect of Rust would you like to explore?";
    } else if (lower.includes('tauri')) {
        response = "Tauri is an excellent framework for building cross-platform desktop and mobile applications using Rust for the backend and web technologies for the frontend. It's lightweight and secure!";
    } else {
        response = `That's an interesting question. In a production app, I would connect to an AI service to provide a meaningful response. For now, this is a mock response.`;
    }

    return [
        { id: Date.now().toString(), role: 'user', content: userMessage, timestamp: new Date().toISOString() },
        { id: (Date.now() + 1).toString(), role: 'assistant', content: response, timestamp: new Date().toISOString() },
    ];
}

function renderMessages(messages) {
    // Clear existing messages
    elements.messagesContainer.querySelectorAll('.message').forEach(m => m.remove());
    elements.welcomeMessage.style.display = messages.length === 0 ? 'flex' : 'none';

    messages.forEach(msg => appendMessage(msg));
    scrollToBottom();
}

function appendMessage(message) {
    const messageEl = document.createElement('div');
    messageEl.className = `message ${message.role}`;

    const avatar = message.role === 'user' ? 'U' : 'K';
    const role = message.role === 'user' ? 'You' : 'Kasate';

    messageEl.innerHTML = `
        <div class="message-avatar">${avatar}</div>
        <div class="message-content">
            <div class="message-role">${role}</div>
            <div class="message-text">${formatMessage(message.content)}</div>
        </div>
    `;

    elements.messagesContainer.appendChild(messageEl);
    scrollToBottom();
}

function formatMessage(content) {
    // Basic markdown-like formatting
    let formatted = escapeHtml(content);

    // Code blocks
    formatted = formatted.replace(/```(\w*)\n([\s\S]*?)```/g, '<pre><code>$2</code></pre>');

    // Inline code
    formatted = formatted.replace(/`([^`]+)`/g, '<code>$1</code>');

    // Bold
    formatted = formatted.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');

    // Line breaks
    formatted = formatted.replace(/\n/g, '<br>');

    return formatted;
}

function scrollToBottom() {
    elements.messagesContainer.scrollTop = elements.messagesContainer.scrollHeight;
}

// Voice
function setupVoice() {
    elements.voiceBtn.addEventListener('click', toggleVoice);
}

function toggleVoice() {
    isListening = !isListening;

    if (isListening) {
        startListening();
    } else {
        stopListening();
    }
}

function startListening() {
    elements.voiceBtn.classList.add('active');
    elements.halEye.classList.add('active');
    elements.voiceStatus.classList.add('listening');
    elements.voiceStatus.querySelector('.status-text').textContent = 'Listening...';
    elements.transcriptText.classList.add('active');
    elements.transcriptText.textContent = 'Speak now...';

    // In a real app, we'd use the Web Speech API or a Tauri plugin
    // This is a mock implementation
    setTimeout(() => {
        if (isListening) {
            elements.transcriptText.textContent = '"What can you help me with today?"';

            // Simulate AI response
            setTimeout(() => {
                if (isListening) {
                    elements.voiceStatus.classList.remove('listening');
                    elements.voiceStatus.classList.add('speaking');
                    elements.voiceStatus.querySelector('.status-text').textContent = 'Speaking...';
                    elements.transcriptText.textContent = "I can help you with programming, answer questions, assist with writing, and much more. What would you like to explore?";

                    setTimeout(() => {
                        stopListening();
                    }, 3000);
                }
            }, 2000);
        }
    }, 2000);
}

function stopListening() {
    isListening = false;
    elements.voiceBtn.classList.remove('active');
    elements.halEye.classList.remove('active');
    elements.voiceStatus.classList.remove('listening', 'speaking');
    elements.voiceStatus.querySelector('.status-text').textContent = 'Press to speak';
}

// History
function renderHistoryList() {
    if (conversations.length === 0) {
        elements.historyList.innerHTML = `
            <div class="history-empty">
                <div class="history-empty-icon">
                    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"></circle>
                        <polyline points="12 6 12 12 16 14"></polyline>
                    </svg>
                </div>
                <h3>No conversations yet</h3>
                <p>Start a new chat to see your history here</p>
            </div>
        `;
        return;
    }

    // Group by date (simplified - just "Today" and "Earlier")
    const today = new Date().toDateString();

    const todayConvs = conversations.filter(c => new Date(c.created_at).toDateString() === today);
    const earlierConvs = conversations.filter(c => new Date(c.created_at).toDateString() !== today);

    let html = '';

    if (todayConvs.length > 0) {
        html += `
            <div class="history-group">
                <div class="history-group-title">Today</div>
                ${todayConvs.map(renderHistoryItem).join('')}
            </div>
        `;
    }

    if (earlierConvs.length > 0) {
        html += `
            <div class="history-group">
                <div class="history-group-title">Earlier</div>
                ${earlierConvs.map(renderHistoryItem).join('')}
            </div>
        `;
    }

    elements.historyList.innerHTML = html;

    // Add click handlers
    document.querySelectorAll('.history-item').forEach(item => {
        item.addEventListener('click', (e) => {
            if (!e.target.closest('.history-action-btn')) {
                selectConversation(item.dataset.id);
            }
        });
    });

    document.querySelectorAll('.history-action-btn.delete').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            deleteConversation(btn.dataset.id);
        });
    });
}

function renderHistoryItem(conv) {
    const lastMessage = conv.messages[conv.messages.length - 1];
    const preview = lastMessage ? lastMessage.content.slice(0, 100) : 'No messages yet';

    return `
        <div class="history-item" data-id="${conv.id}">
            <div class="history-item-icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
                </svg>
            </div>
            <div class="history-item-content">
                <div class="history-item-title">${escapeHtml(conv.title)}</div>
                <div class="history-item-preview">${escapeHtml(preview)}</div>
                <div class="history-item-meta">
                    <span class="history-item-messages">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
                        </svg>
                        ${conv.messages.length} messages
                    </span>
                </div>
            </div>
            <div class="history-item-actions">
                <button class="history-action-btn delete" data-id="${conv.id}">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="3 6 5 6 21 6"></polyline>
                        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    </svg>
                </button>
            </div>
        </div>
    `;
}

// Search functionality
elements.historySearch?.addEventListener('input', (e) => {
    const query = e.target.value.toLowerCase();
    document.querySelectorAll('.history-item').forEach(item => {
        const title = item.querySelector('.history-item-title').textContent.toLowerCase();
        const preview = item.querySelector('.history-item-preview').textContent.toLowerCase();
        const matches = title.includes(query) || preview.includes(query);
        item.style.display = matches ? 'flex' : 'none';
    });
});

// Utility
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
