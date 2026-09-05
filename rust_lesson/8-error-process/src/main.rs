use axum::{
    routing::{get, post},
    Router,
    extract::Form,
    response::Html,
};
use serde::Deserialize;

#[tokio::main]
async fn main() {
    // ルーティングの設定
    let app = Router::new()
        .route("/", get(index))
        .route("/greet", post(greet));

    // サーバーの起動
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("サーバーを http://localhost:3000 で起動中...");
    axum::serve(listener, app).await.unwrap();
}

// トップページ（HTMLフォームを返す）
async fn index() -> Html<&'static str> {
    Html(r##"
        <!DOCTYPE html>
        <html lang="ja">
        <head>
            <meta charset="UTF-8">
            <title>Rust Web App</title>
            <style>
                body {
                    font-family: sans-serif;
                    max-width: 600px;
                    margin: 50px auto;
                    padding: 20px;
                    background: #f5f5f5;
                }
                h1 { color: #333; }
                input, button {
                    padding: 10px;
                    font-size: 16px;
                    margin-top: 10px;
                }
                input {
                    width: 60%;
                    border: 1px solid #ccc;
                    border-radius: 4px;
                }
                button {
                    background: #007bff;
                    color: white;
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                }
                button:hover { background: #0056b3; }
            </style>
        </head>
        <body>
            <h1>Rust + Axum Web アプリ</h1>
            <p>名前を入力してください:</p>
            <form action="/greet" method="post">
                <input type="text" name="name" placeholder="あなたの名前" required>
                <button type="submit">送信</button>
            </form>
        </body>
        </html>
    "##)
}

// フォームデータの受け取り用構造体
#[derive(Deserialize)]
struct GreetForm {
    name: String,
}

// フォーム送信後の挨拶ページ
async fn greet(Form(form): Form<GreetForm>) -> Html<String> {
    Html(format!(r##"
        <!DOCTYPE html>
        <html lang="ja">
        <head>
            <meta charset="UTF-8">
            <title>挨拶</title>
            <style>
                body {{
                    font-family: sans-serif;
                    max-width: 600px;
                    margin: 50px auto;
                    padding: 20px;
                    background: #f5f5f5;
                }}
                h1 {{ color: #007bff; }}
                a {{ color: #007bff; text-decoration: none; }}
                a:hover {{ text-decoration: underline; }}
            </style>
        </head>
        <body>
            <h1>こんにちは、{} さん！</h1>
            <p>Rustで動作するWebアプリからの挨拶です。</p>
            <a href="/">← 戻る</a>
        </body>
        </html>
    "##, axum::extract:: rejection::html_escape(&form.name)))
}
