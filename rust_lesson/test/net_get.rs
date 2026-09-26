use reqwest;
use scraper::{Html, Selector};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. SQLiteデータベース接続（ファイルが存在しない場合は自動作成）
    let db_url = "sqlite://scraped_data.db?mode=rwc";
    let pool = SqlitePoolOptions::new()
        .connect(db_url)
        .await?;

    // 2. テーブルの作成
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            url TEXT NOT NULL
        );"
    )
    .execute(&pool)
    .await?;

    // 3. Webページの取得
    let target_url = "https://news.ycombinator.com/"; // サンプル対象（Hacker News）
    let response = reqwest::get(target_url).await?.text().await?;

    // 4. HTMLの解析（HTMLドキュメントとCSSセレクタのパース）
    let document = Html::parse_document(&response);
    let title_selector = Selector::parse(".titleline > a").unwrap();

    println!("--- データを取得してデータベースへ保存中 ---");

    // 5. 該当要素の抽出とDB挿入
    for element in document.select(&title_selector) {
        let title = element.text().collect::<Vec<_>>().join("");
        if let Some(url) = element.value().attr("href") {
            // DBへデータを挿入
            insert_article(&pool, &title, url).await?;
            println!("保存完了: {} ({})", title, url);
        }
    }

    println!("すべてのデータ処理が完了しました。");
    Ok(())
}

// データベースへの挿入関数
async fn insert_article(pool: &SqlitePool, title: &str, url: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO articles (title, url) VALUES ($1, $2)"
    )
    .bind(title)
    .bind(url)
    .execute(pool)
    .await?;

    Ok(())
}