use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::Json, payload::PlainText, OpenApi, OpenApiService};
use sqlite::Connection;

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }

    #[oai(path = "/todo", method = "post")]
    async fn add_todo(&self, text: PlainText<String>) -> PlainText<String> {
        let conn = init_db().expect("Failed to open database");
        match add_todo(&conn, &text.0) {
            Ok(_) => PlainText("Todo added successfully".to_string()),
            Err(e) => PlainText(format!("Error adding todo: {}", e)),
        }
    }

    #[oai(path = "/todos", method = "get")]
    async fn list_todos_api(&self) -> Json<Vec<(i64, String, bool)>> {
        let conn = init_db().expect("Failed to open database");
        let todos = list_todos(&conn).unwrap_or_default();
        Json(todos)
    }

    #[oai(path = "/todo/:id/complete", method = "post")]
    async fn complete_todo_api(&self, id: i64) -> PlainText<String> {
        let conn = init_db().expect("Failed to open database");
        match complete_todo(&conn, id) {
            Ok(_) => PlainText("Todo completed successfully".to_string()),
            Err(e) => PlainText(format!("Error completing todo: {}", e)),
        }
    }
}

fn init_db() -> Result<Connection, sqlite::Error> {
    let conn = sqlite::open("todos.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            completed BOOLEAN NOT NULL DEFAULT 0
        )",
    )?;
    Ok(conn)
}

fn add_todo(conn: &Connection, text: &str) -> Result<(), sqlite::Error> {
    let mut statement = conn.prepare("INSERT INTO todos (text) VALUES (?)")?;
    statement.bind((1, text))?;
    statement.next()?;
    Ok(())
}

fn list_todos(conn: &Connection) -> Result<Vec<(i64, String, bool)>, sqlite::Error> {
    let mut statement = conn.prepare("SELECT id, text, completed FROM todos")?;
    let mut todos = Vec::new();
    while let Ok(sqlite::State::Row) = statement.next() {
        let id: i64 = statement.read(0)?;
        let text: String = statement.read(1)?;
        let completed: i64 = statement.read(2)?;
        todos.push((id, text, completed != 0));
    }
    Ok(todos)
}

fn complete_todo(conn: &Connection, id: i64) -> Result<(), sqlite::Error> {
    let mut statement = conn.prepare("UPDATE todos SET completed = 1 WHERE id = ?")?;
    statement.bind((1, id))?;
    statement.next()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let api = Api::new();
    let api_service =
        OpenApiService::new(api, "Todo App", "1.0").server("http://localhost:3000/api");
    let json = api_service.spec();
    std::fs::write("openapi.json", json).unwrap();
    let ui = api_service.swagger_ui();
    let app = Route::new().nest("/api", api_service).nest("/", ui);

    poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
