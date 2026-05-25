use ratatui_kit::Store;

pub struct TodoItem {
    pub id: u32,
    pub title: String,
    pub completed: bool,
}

pub struct TodoList {
    pub items: Vec<TodoItem>,
}

pub struct TaskItem {
    pub id: u32,
    pub title: String,
    pub completed: bool,
}

pub struct TaskList {
    pub items: Vec<TaskItem>,
}

#[derive(Store)]
pub struct GlobalStore {
    pub llm_name: String,
    pub current_chat: String,
    pub cache_size: f64,
    pub context_size: f64,
    pub model_context_window: f64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub current_chat_count: u32,
    pub todo_list: TodoList,
    pub task_list: TaskList,
    pub input_buffer: String,
}

impl Default for GlobalStore {
    fn default() -> Self {
        Self {
            llm_name: String::new(),
            current_chat: String::new(),
            cache_size: 0.0,
            context_size: 0.0,
            model_context_window: 0.0,
            input_tokens: 0,
            output_tokens: 0,
            current_chat_count: 0,
            todo_list: TodoList {
                items: Vec::new(),
            },
            task_list: TaskList {
                items: Vec::new(),
            },
            input_buffer: String::new(),
        }
    }
}