// 1. エラー型の定義 (Debugをderiveしておくと便利です)
#[derive(Debug)]
enum RegistrationError {
    todo!("ここに列挙子を追加してください")
}

// 2. バリデーション関数の実装
fn register_user(name: &str, age: i32) -> Result<(), RegistrationError> {
    // ここに条件分岐を実装してください
    todo!()
}

fn main() {
    let test_name = "";
    let test_age = 25;

    // 3. match式によるエラーハンドリング
    match register_user(test_name, test_age) {
        todo!("各ケース（Ok, Err）に応じた処理を書いてください")
    }
}